// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) fname, algo
use clap::{crate_version, value_parser, Arg, ArgAction, Command};
use hex::decode;
use hex::encode;
use std::error::Error;
use std::ffi::OsStr;
use std::fmt::Display;
use std::fs::File;
use std::io::{self, stdin, stdout, BufReader, Read, Write};
use std::iter;
use std::path::Path;
use uucore::{
    encoding,
    error::{FromIo, UError, UResult, USimpleError},
    format_usage, help_about, help_section, help_usage, show,
    sum::{
        div_ceil, Blake2b, Digest, DigestWriter, Md5, Sha1, Sha224, Sha256, Sha384, Sha512, Sm3,
        BSD, CRC, SYSV,
    },
};

const USAGE: &str = help_usage!("cksum.md");
const ABOUT: &str = help_about!("cksum.md");
const AFTER_HELP: &str = help_section!("after help", "cksum.md");

const ALGORITHM_OPTIONS_SYSV: &str = "sysv";
const ALGORITHM_OPTIONS_BSD: &str = "bsd";
const ALGORITHM_OPTIONS_CRC: &str = "crc";
const ALGORITHM_OPTIONS_MD5: &str = "md5";
const ALGORITHM_OPTIONS_SHA1: &str = "sha1";
const ALGORITHM_OPTIONS_SHA224: &str = "sha224";
const ALGORITHM_OPTIONS_SHA256: &str = "sha256";
const ALGORITHM_OPTIONS_SHA384: &str = "sha384";
const ALGORITHM_OPTIONS_SHA512: &str = "sha512";
const ALGORITHM_OPTIONS_BLAKE2B: &str = "blake2b";
const ALGORITHM_OPTIONS_SM3: &str = "sm3";

#[derive(Debug)]
enum CkSumError {
    RawMultipleFiles,
}

#[derive(Debug, PartialEq)]
enum OutputFormat {
    Hexadecimal,
    Raw,
    Base64,
}

impl UError for CkSumError {
    fn code(&self) -> i32 {
        match self {
            Self::RawMultipleFiles => 1,
        }
    }
}

impl Error for CkSumError {}

impl Display for CkSumError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RawMultipleFiles => {
                write!(f, "the --raw option is not supported with multiple files")
            }
        }
    }
}

fn detect_algo(
    program: &str,
    length: Option<usize>,
) -> (&'static str, Box<dyn Digest + 'static>, usize) {
    match program {
        ALGORITHM_OPTIONS_SYSV => (
            ALGORITHM_OPTIONS_SYSV,
            Box::new(SYSV::new()) as Box<dyn Digest>,
            512,
        ),
        ALGORITHM_OPTIONS_BSD => (
            ALGORITHM_OPTIONS_BSD,
            Box::new(BSD::new()) as Box<dyn Digest>,
            1024,
        ),
        ALGORITHM_OPTIONS_CRC => (
            ALGORITHM_OPTIONS_CRC,
            Box::new(CRC::new()) as Box<dyn Digest>,
            256,
        ),
        ALGORITHM_OPTIONS_MD5 => (
            ALGORITHM_OPTIONS_MD5,
            Box::new(Md5::new()) as Box<dyn Digest>,
            128,
        ),
        ALGORITHM_OPTIONS_SHA1 => (
            ALGORITHM_OPTIONS_SHA1,
            Box::new(Sha1::new()) as Box<dyn Digest>,
            160,
        ),
        ALGORITHM_OPTIONS_SHA224 => (
            ALGORITHM_OPTIONS_SHA224,
            Box::new(Sha224::new()) as Box<dyn Digest>,
            224,
        ),
        ALGORITHM_OPTIONS_SHA256 => (
            ALGORITHM_OPTIONS_SHA256,
            Box::new(Sha256::new()) as Box<dyn Digest>,
            256,
        ),
        ALGORITHM_OPTIONS_SHA384 => (
            ALGORITHM_OPTIONS_SHA384,
            Box::new(Sha384::new()) as Box<dyn Digest>,
            384,
        ),
        ALGORITHM_OPTIONS_SHA512 => (
            ALGORITHM_OPTIONS_SHA512,
            Box::new(Sha512::new()) as Box<dyn Digest>,
            512,
        ),
        ALGORITHM_OPTIONS_BLAKE2B => (
            ALGORITHM_OPTIONS_BLAKE2B,
            Box::new(if let Some(length) = length {
                Blake2b::with_output_bytes(length)
            } else {
                Blake2b::new()
            }) as Box<dyn Digest>,
            512,
        ),
        ALGORITHM_OPTIONS_SM3 => (
            ALGORITHM_OPTIONS_SM3,
            Box::new(Sm3::new()) as Box<dyn Digest>,
            512,
        ),
        _ => unreachable!("unknown algorithm: clap should have prevented this case"),
    }
}

struct Options {
    algo_name: &'static str,
    digest: Box<dyn Digest + 'static>,
    output_bits: usize,
    untagged: bool,
    length: Option<usize>,
    output_format: OutputFormat,
    binary: bool,
}

/// Calculate checksum
///
/// # Arguments
///
/// * `options` - CLI options for the assigning checksum algorithm
/// * `files` - A iterator of OsStr which is a bunch of files that are using for calculating checksum
#[allow(clippy::cognitive_complexity)]
fn cksum<'a, I>(mut options: Options, files: I) -> UResult<()>
where
    I: Iterator<Item = &'a OsStr>,
{
    let files: Vec<_> = files.collect();
    if options.output_format == OutputFormat::Raw && files.len() > 1 {
        return Err(Box::new(CkSumError::RawMultipleFiles));
    }

    for filename in files {
        let filename = Path::new(filename);
        let stdin_buf;
        let file_buf;
        let not_file = filename == OsStr::new("-");

        // Handle the file input
        let mut file = BufReader::new(if not_file {
            stdin_buf = stdin();
            Box::new(stdin_buf) as Box<dyn Read>
        } else if filename.is_dir() {
            Box::new(BufReader::new(io::empty())) as Box<dyn Read>
        } else {
            file_buf = match File::open(filename) {
                Ok(file) => file,
                Err(err) => {
                    show!(err.map_err_context(|| filename.to_string_lossy().to_string()));
                    continue;
                }
            };
            Box::new(file_buf) as Box<dyn Read>
        });

        let (sum_hex, sz) = digest_read(&mut options.digest, &mut file, options.output_bits)
            .map_err_context(|| "failed to read input".to_string())?;
        if filename.is_dir() {
            show!(USimpleError::new(
                1,
                format!("{}: Is a directory", filename.display())
            ));
            continue;
        }
        let sum = match options.output_format {
            OutputFormat::Raw => {
                let bytes = match options.algo_name {
                    ALGORITHM_OPTIONS_CRC => sum_hex.parse::<u32>().unwrap().to_be_bytes().to_vec(),
                    ALGORITHM_OPTIONS_SYSV | ALGORITHM_OPTIONS_BSD => {
                        sum_hex.parse::<u16>().unwrap().to_be_bytes().to_vec()
                    }
                    _ => decode(sum_hex).unwrap(),
                };
                // Cannot handle multiple files anyway, output immediately.
                stdout().write_all(&bytes)?;
                return Ok(());
            }
            OutputFormat::Hexadecimal => sum_hex,
            OutputFormat::Base64 => match options.algo_name {
                ALGORITHM_OPTIONS_CRC | ALGORITHM_OPTIONS_SYSV | ALGORITHM_OPTIONS_BSD => sum_hex,
                _ => encoding::encode(encoding::Format::Base64, &decode(sum_hex).unwrap()).unwrap(),
            },
        };
        // The BSD checksum output is 5 digit integer
        let bsd_width = 5;
        match (options.algo_name, not_file) {
            (ALGORITHM_OPTIONS_SYSV, true) => println!(
                "{} {}",
                sum.parse::<u16>().unwrap(),
                div_ceil(sz, options.output_bits)
            ),
            (ALGORITHM_OPTIONS_SYSV, false) => println!(
                "{} {} {}",
                sum.parse::<u16>().unwrap(),
                div_ceil(sz, options.output_bits),
                filename.display()
            ),
            (ALGORITHM_OPTIONS_BSD, true) => println!(
                "{:0bsd_width$} {:bsd_width$}",
                sum.parse::<u16>().unwrap(),
                div_ceil(sz, options.output_bits)
            ),
            (ALGORITHM_OPTIONS_BSD, false) => println!(
                "{:0bsd_width$} {:bsd_width$} {}",
                sum.parse::<u16>().unwrap(),
                div_ceil(sz, options.output_bits),
                filename.display()
            ),
            (ALGORITHM_OPTIONS_CRC, true) => println!("{sum} {sz}"),
            (ALGORITHM_OPTIONS_CRC, false) => println!("{sum} {sz} {}", filename.display()),
            (ALGORITHM_OPTIONS_BLAKE2B, _) if !options.untagged => {
                if let Some(length) = options.length {
                    // Multiply by 8 here, as we want to print the length in bits.
                    println!("BLAKE2b-{} ({}) = {sum}", length * 8, filename.display());
                } else {
                    println!("BLAKE2b ({}) = {sum}", filename.display());
                }
            }
            _ => {
                if options.untagged {
                    let prefix = if options.binary { "*" } else { " " };
                    println!("{sum} {prefix}{}", filename.display());
                } else {
                    println!(
                        "{} ({}) = {sum}",
                        options.algo_name.to_ascii_uppercase(),
                        filename.display()
                    );
                }
            }
        }
    }

    Ok(())
}

fn digest_read<T: Read>(
    digest: &mut Box<dyn Digest>,
    reader: &mut BufReader<T>,
    output_bits: usize,
) -> io::Result<(String, usize)> {
    digest.reset();

    // Read bytes from `reader` and write those bytes to `digest`.
    //
    // If `binary` is `false` and the operating system is Windows, then
    // `DigestWriter` replaces "\r\n" with "\n" before it writes the
    // bytes into `digest`. Otherwise, it just inserts the bytes as-is.
    //
    // In order to support replacing "\r\n", we must call `finalize()`
    // in order to support the possibility that the last character read
    // from the reader was "\r". (This character gets buffered by
    // `DigestWriter` and only written if the following character is
    // "\n". But when "\r" is the last character read, we need to force
    // it to be written.)
    let mut digest_writer = DigestWriter::new(digest, true);
    let output_size = std::io::copy(reader, &mut digest_writer)? as usize;
    digest_writer.finalize();

    if digest.output_bits() > 0 {
        Ok((digest.result_str(), output_size))
    } else {
        // Assume it's SHAKE.  result_str() doesn't work with shake (as of 8/30/2016)
        let mut bytes = vec![0; (output_bits + 7) / 8];
        digest.hash_finalize(&mut bytes);
        Ok((encode(bytes), output_size))
    }
}

mod options {
    pub const ALGORITHM: &str = "algorithm";
    pub const FILE: &str = "file";
    pub const UNTAGGED: &str = "untagged";
    pub const TAG: &str = "tag";
    pub const LENGTH: &str = "length";
    pub const RAW: &str = "raw";
    pub const BASE64: &str = "base64";
    pub const CHECK: &str = "check";
    // for legacy compat reasons
    pub const TEXT: &str = "text";
    pub const BINARY: &str = "binary";
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().try_get_matches_from(args)?;

    let algo_name: &str = match matches.get_one::<String>(options::ALGORITHM) {
        Some(v) => v,
        None => ALGORITHM_OPTIONS_CRC,
    };

    let input_length = matches.get_one::<usize>(options::LENGTH);
    let check = matches.get_flag(options::CHECK);

    let length = if let Some(length) = input_length {
        match length.to_owned() {
            0 => None,
            n if n % 8 != 0 => {
                // GNU's implementation seem to use these quotation marks
                // in their error messages, so we do the same.
                uucore::show_error!("invalid length: \u{2018}{length}\u{2019}");
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "length is not a multiple of 8",
                )
                .into());
            }
            n if n > 512 => {
                uucore::show_error!("invalid length: \u{2018}{length}\u{2019}");

                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "maximum digest length for \u{2018}BLAKE2b\u{2019} is 512 bits",
                )
                .into());
            }
            n => {
                if algo_name != ALGORITHM_OPTIONS_BLAKE2B {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--length is only supported with --algorithm=blake2b",
                    )
                    .into());
                }

                // Divide by 8, as our blake2b implementation expects bytes
                // instead of bits.
                Some(n / 8)
            }
        }
    } else {
        None
    };

    if algo_name == "bsd" && check {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "--check is not supported with --algorithm={bsd,sysv,crc}",
        )
        .into());
    }

    let untagged: bool = matches.get_flag(options::UNTAGGED);
    let tag: bool = matches.get_flag(options::TAG);

    let binary = if untagged && tag {
        false
    } else {
        matches.get_flag(options::BINARY)
    };

    let (name, algo, bits) = detect_algo(algo_name, length);

    let output_format = if matches.get_flag(options::RAW) {
        OutputFormat::Raw
    } else if matches.get_flag(options::BASE64) {
        OutputFormat::Base64
    } else {
        OutputFormat::Hexadecimal
    };

    let opts = Options {
        algo_name: name,
        digest: algo,
        output_bits: bits,
        length,
        untagged,
        output_format,
        binary,
    };

    match matches.get_many::<String>(options::FILE) {
        Some(files) => cksum(opts, files.map(OsStr::new))?,
        None => cksum(opts, iter::once(OsStr::new("-")))?,
    };

    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .args_override_self(true)
        .arg(
            Arg::new(options::FILE)
                .hide(true)
                .action(clap::ArgAction::Append)
                .value_hint(clap::ValueHint::FilePath),
        )
        .arg(
            Arg::new(options::ALGORITHM)
                .long(options::ALGORITHM)
                .short('a')
                .help("select the digest type to use. See DIGEST below")
                .value_name("ALGORITHM")
                .value_parser([
                    ALGORITHM_OPTIONS_SYSV,
                    ALGORITHM_OPTIONS_BSD,
                    ALGORITHM_OPTIONS_CRC,
                    ALGORITHM_OPTIONS_MD5,
                    ALGORITHM_OPTIONS_SHA1,
                    ALGORITHM_OPTIONS_SHA224,
                    ALGORITHM_OPTIONS_SHA256,
                    ALGORITHM_OPTIONS_SHA384,
                    ALGORITHM_OPTIONS_SHA512,
                    ALGORITHM_OPTIONS_BLAKE2B,
                    ALGORITHM_OPTIONS_SM3,
                ]),
        )
        .arg(
            Arg::new(options::UNTAGGED)
                .long(options::UNTAGGED)
                .help("create a reversed style checksum, without digest type")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::TAG)
                .long(options::TAG)
                .help("create a BSD style checksum, undo --untagged (default)")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::LENGTH)
                .long(options::LENGTH)
                .value_parser(value_parser!(usize))
                .short('l')
                .help(
                    "digest length in bits; must not exceed the max for the blake2 algorithm \
                    and must be a multiple of 8",
                )
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new(options::RAW)
                .long(options::RAW)
                .help("emit a raw binary digest, not hexadecimal")
                .action(ArgAction::SetTrue),
        )
        /*.arg(
            Arg::new(options::STRICT)
                .long(options::STRICT)
                .help("exit non-zero for improperly formatted checksum lines")
                .action(ArgAction::SetTrue),
        )*/
        .arg(
            Arg::new(options::CHECK)
                .short('c')
                .long(options::CHECK)
                .help("read hashsums from the FILEs and check them")
                .action(ArgAction::SetTrue)
                .conflicts_with("tag"),
        )
        .arg(
            Arg::new(options::BASE64)
                .long(options::BASE64)
                .help("emit a base64 digest, not hexadecimal")
                .action(ArgAction::SetTrue)
                // Even though this could easily just override an earlier '--raw',
                // GNU cksum does not permit these flags to be combined:
                .conflicts_with(options::RAW),
        )
        .arg(
            Arg::new(options::TEXT)
                .long(options::TEXT)
                .short('t')
                .hide(true) // for legacy compatibility, no action
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::BINARY)
                .long(options::BINARY)
                .short('b')
                .hide(true) // for legacy compatibility, no action
                .action(ArgAction::SetTrue),
        )
        .after_help(AFTER_HELP)
}
