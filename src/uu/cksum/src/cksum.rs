// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) fname, algo
use clap::{crate_version, Arg, ArgAction, Command};
use hex::encode;
use regex::{Captures, Regex};
use std::error::Error;
use std::ffi::OsStr;
use std::fs::File;
use std::io::{self, stdin, BufRead, BufReader, Read};
use std::iter;
use std::num::ParseIntError;
use std::path::Path;
use uucore::display::Quotable;
use uucore::error::UError;
use uucore::show_warning;
use uucore::{
    error::{FromIo, UResult},
    format_usage, help_about, help_section, help_usage,
    sum::{
        div_ceil, Blake2b, Digest, DigestWriter, Md5, Sha1, Sha224, Sha256, Sha384, Sha512, Sm3,
        BSD, CRC, SYSV,
    },
};

const USAGE: &str = help_usage!("cksum.md");
const ABOUT: &str = help_about!("cksum.md");
const AFTER_HELP: &str = help_section!("after help", "cksum.md");

mod algorithm {
    pub const SYSV: &str = "sysv";
    pub const BSD: &str = "bsd";
    pub const CRC: &str = "crc";
    pub const MD5: &str = "md5";
    pub const SHA1: &str = "sha1";
    pub const SHA224: &str = "sha224";
    pub const SHA256: &str = "sha256";
    pub const SHA384: &str = "sha384";
    pub const SHA512: &str = "sha512";
    pub const BLAKE2B: &str = "blake2b";
    pub const SM3: &str = "sm3";
}

mod options {
    // cksum
    pub const ALGORITHM: &str = "algorithm";
    pub const FILE: &str = "file";
    pub const UNTAGGED: &str = "untagged";

    // common
    pub const BINARY: &'static str = "binary";
    pub const TEXT: &'static str = "text";
    pub const CHECK: &'static str = "check";
    pub const TAG: &'static str = "tag";
    pub const STATUS: &'static str = "status";
    pub const QUIET: &'static str = "quiet";
    pub const STRICT: &'static str = "strict";
    pub const WARN: &'static str = "warn";
    pub const ZERO: &'static str = "zero";

    // length argument for variable length utils
    pub const LENGTH: &'static str = "length";
}

const BINARY_FLAG_DEFAULT: bool = cfg!(windows);

#[derive(Debug)]
enum CksumError {
    InvalidFormat,
}

impl Error for CksumError {}
impl UError for CksumError {}

impl std::fmt::Display for CksumError {
    fn fmt(&self, _f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::InvalidFormat => Ok(()),
        }
    }
}

fn detect_algo(program: &str) -> (&'static str, Box<dyn Digest + 'static>, usize) {
    use algorithm::*;
    match program {
        SYSV => (SYSV, Box::new(SYSV::new()) as Box<dyn Digest>, 512),
        BSD => (BSD, Box::new(BSD::new()) as Box<dyn Digest>, 1024),
        CRC => (CRC, Box::new(CRC::new()) as Box<dyn Digest>, 256),
        MD5 => (MD5, Box::new(Md5::new()) as Box<dyn Digest>, 128),
        SHA1 => (SHA1, Box::new(Sha1::new()) as Box<dyn Digest>, 160),
        SHA224 => (SHA224, Box::new(Sha224::new()) as Box<dyn Digest>, 224),
        SHA256 => (SHA256, Box::new(Sha256::new()) as Box<dyn Digest>, 256),
        SHA384 => (SHA384, Box::new(Sha384::new()) as Box<dyn Digest>, 384),
        SHA512 => (SHA512, Box::new(Sha512::new()) as Box<dyn Digest>, 512),
        BLAKE2B => (BLAKE2B, Box::new(Blake2b::new()) as Box<dyn Digest>, 512),
        SM3 => (SM3, Box::new(Sm3::new()) as Box<dyn Digest>, 512),
        _ => unreachable!("unknown algorithm: clap should have prevented this case"),
    }
}

struct Options {
    // cksum
    algo_name: &'static str,
    digest: Box<dyn Digest + 'static>,
    output_bits: usize,

    // common
    binary: bool,
    check: bool,
    tag: bool,
    status: bool,
    quiet: bool,
    strict: bool,
    warn: bool,
    // zero is unimplemented
    _zero: bool,
}

/// Calculate checksum
///
/// # Arguments
///
/// * `options` - CLI options for the assigning checksum algorithm
/// * `files` - A iterator of OsStr which is a bunch of files that are using for calculating checksum
#[allow(clippy::cognitive_complexity)]
fn cksum<'a, I>(options: Options, files: I) -> UResult<()>
where
    I: Iterator<Item = &'a OsStr>,
{
    if options.check {
        cksum_check(options, files)
    } else {
        cksum_print(options, files)
    }
}

/// Creates a Regex for parsing lines based on the given format.
/// The default value of `gnu_re` created with this function has to be recreated
/// after the initial line has been parsed, as this line dictates the format
/// for the rest of them, and mixing of formats is disallowed.
fn gnu_re_template(bytes_marker: &str, format_marker: &str) -> Regex {
    Regex::new(&format!(
        r"^(?P<digest>[a-fA-F0-9]{bytes_marker}) {format_marker}(?P<fileName>.*)"
    ))
    .expect("internal error: invalid regex")
}

fn handle_captures(
    caps: &Captures,
    bytes_marker: &str,
    bsd_reversed: &mut Option<bool>,
    gnu_re: &mut Regex,
) -> (String, String, bool) {
    if bsd_reversed.is_none() {
        let is_bsd_reversed = caps.name("binary").is_none();
        let format_marker = if is_bsd_reversed {
            ""
        } else {
            r"(?P<binary>[ \*])"
        }
        .to_string();

        *bsd_reversed = Some(is_bsd_reversed);
        *gnu_re = gnu_re_template(bytes_marker, &format_marker);
    }

    (
        caps.name("fileName").unwrap().as_str().to_string(),
        caps.name("digest").unwrap().as_str().to_ascii_lowercase(),
        if *bsd_reversed == Some(false) {
            caps.name("binary").unwrap().as_str() == "*"
        } else {
            false
        },
    )
}

fn cksum_check<'a, I>(mut options: Options, files: I) -> UResult<()>
where
    I: Iterator<Item = &'a OsStr>,
{
    // Set up Regexes for line validation and parsing
    //
    // First, we compute the number of bytes we expect to be in
    // the digest string. If the algorithm has a variable number
    // of output bits, then we use the `+` modifier in the
    // regular expression, otherwise we use the `{n}` modifier,
    // where `n` is the number of bytes.
    let bytes = options.digest.output_bits() / 4;
    let bytes_marker = if bytes > 0 {
        format!("{{{bytes}}}")
    } else {
        "+".to_string()
    };

    // BSD reversed mode format is similar to the default mode, but doesn’t use
    // a character to distinguish binary and text modes.
    let mut bsd_reversed = None;

    let mut gnu_re = gnu_re_template(&bytes_marker, r"(?P<binary>[ \*])?");
    let bsd_re = Regex::new(&format!(
        r"^{algorithm} \((?P<fileName>.*)\) = (?P<digest>[a-fA-F0-9]{digest_size})",
        algorithm = options.algo_name,
        digest_size = bytes_marker,
    ))
    .expect("internal error: invalid regex");

    // Keep track of the number of errors to report at the end
    let mut num_bad_format_errors = 0;
    let mut num_failed_checksums = 0;
    let mut num_failed_to_open = 0;

    for filename in files {
        let buffer = open_file(filename)?;
        for (i, maybe_line) in buffer.lines().enumerate() {
            let line = match maybe_line {
                Ok(l) => l,
                Err(e) => return Err(e.map_err_context(|| "failed to read file".to_string())),
            };
            let (ck_filename, sum, binary_check) = match gnu_re.captures(&line) {
                Some(caps) => handle_captures(&caps, &bytes_marker, &mut bsd_reversed, &mut gnu_re),
                None => match bsd_re.captures(&line) {
                    Some(caps) => (
                        caps.name("fileName").unwrap().as_str().to_string(),
                        caps.name("digest").unwrap().as_str().to_ascii_lowercase(),
                        true,
                    ),
                    None => {
                        num_bad_format_errors += 1;
                        if options.strict {
                            return Err(CksumError::InvalidFormat.into());
                        }
                        if options.warn {
                            show_warning!(
                                "{}: {}: improperly formatted {} checksum line",
                                filename.maybe_quote(),
                                i + 1,
                                options.algo_name
                            );
                        }
                        continue;
                    }
                },
            };
            let f = match File::open(ck_filename.clone()) {
                Err(_) => {
                    num_failed_to_open += 1;
                    println!(
                        "{}: {}: No such file or directory",
                        uucore::util_name(),
                        ck_filename
                    );
                    println!("{ck_filename}: FAILED open or read");
                    continue;
                }
                Ok(file) => file,
            };
            let mut ckf = BufReader::new(Box::new(f) as Box<dyn Read>);
            let real_sum = digest_read(
                &mut options.digest,
                &mut ckf,
                binary_check,
                options.output_bits,
            )
            .map_err_context(|| "failed to read input".to_string())?
            .0
            .to_ascii_lowercase();

            // FIXME: Filenames with newlines should be treated specially.
            // GNU appears to replace newlines by \n and backslashes by
            // \\ and prepend a backslash (to the hash or filename) if it did
            // this escaping.
            // Different sorts of output (checking vs outputting hashes) may
            // handle this differently. Compare carefully to GNU.
            // If you can, try to preserve invalid unicode using OsStr(ing)Ext
            // and display it using uucore::display::print_verbatim(). This is
            // easier (and more important) on Unix than on Windows.
            if sum == real_sum {
                if !options.quiet {
                    println!("{ck_filename}: OK");
                }
            } else {
                if !options.status {
                    println!("{ck_filename}: FAILED");
                }
                num_failed_checksums += 1;
            }
        }
    }

    if !options.status {
        match num_bad_format_errors {
            0 => {}
            1 => show_warning!("1 line is improperly formatted"),
            _ => show_warning!("{} lines are improperly formatted", num_bad_format_errors),
        }
        match num_failed_checksums {
            0 => {}
            1 => show_warning!("WARNING: 1 computed checksum did NOT match"),
            _ => show_warning!(
                "WARNING: {} computed checksum did NOT match",
                num_failed_checksums
            ),
        }
        match num_failed_to_open {
            0 => {}
            1 => show_warning!("1 listed file could not be read"),
            _ => show_warning!("{} listed file could not be read", num_failed_to_open),
        }
    }
    Ok(())
}

fn cksum_print<'a, I>(mut options: Options, files: I) -> UResult<()>
where
    I: Iterator<Item = &'a OsStr>,
{
    for filename in files {
        let is_stdin = filename == OsStr::new("-");
        let mut file = open_file(filename)?;
        let path = Path::new(filename);
        let (sum, sz) = digest_read(
            &mut options.digest,
            &mut file,
            options.binary,
            options.output_bits,
        )
        .map_err_context(|| "failed to read input".to_string())?;

        // The BSD checksum output is 5 digit integer
        let bsd_width = 5;
        match (options.algo_name, is_stdin) {
            (algorithm::SYSV, true) => println!(
                "{} {}",
                sum.parse::<u16>().unwrap(),
                div_ceil(sz, options.output_bits)
            ),
            (algorithm::SYSV, false) => println!(
                "{} {} {}",
                sum.parse::<u16>().unwrap(),
                div_ceil(sz, options.output_bits),
                path.display()
            ),
            (algorithm::BSD, true) => println!(
                "{:0bsd_width$} {:bsd_width$}",
                sum.parse::<u16>().unwrap(),
                div_ceil(sz, options.output_bits)
            ),
            (algorithm::BSD, false) => println!(
                "{:0bsd_width$} {:bsd_width$} {}",
                sum.parse::<u16>().unwrap(),
                div_ceil(sz, options.output_bits),
                path.display()
            ),
            (algorithm::CRC, true) => println!("{sum} {sz}"),
            (algorithm::CRC, false) => println!("{sum} {sz} {}", path.display()),
            (algorithm::BLAKE2B, _) if options.tag => {
                println!("BLAKE2b ({}) = {sum}", path.display());
            }
            _ => {
                if options.tag {
                    println!(
                        "{} ({}) = {sum}",
                        options.algo_name.to_ascii_uppercase(),
                        path.display()
                    );
                } else {
                    println!("{sum}  {}", path.display());
                }
            }
        };
    }
    Ok(())
}

fn open_file(filename: &OsStr) -> UResult<BufReader<Box<dyn Read>>> {
    let is_stdin = filename == OsStr::new("-");

    let path = Path::new(filename);
    let reader = if is_stdin {
        let stdin_buf = stdin();
        Box::new(stdin_buf) as Box<dyn Read>
    } else if path.is_dir() {
        Box::new(BufReader::new(io::empty())) as Box<dyn Read>
    } else {
        let file_buf =
            File::open(filename).map_err_context(|| filename.to_str().unwrap().to_string())?;
        Box::new(file_buf) as Box<dyn Read>
    };

    Ok(BufReader::new(reader))
}

fn digest_read<T: Read>(
    digest: &mut Box<dyn Digest>,
    reader: &mut BufReader<T>,
    binary: bool,
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
    let mut digest_writer = DigestWriter::new(digest, binary);
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

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().try_get_matches_from(args)?;

    let algo_name: &str = match matches.get_one::<String>(options::ALGORITHM) {
        Some(v) => v,
        None => algorithm::CRC,
    };

    let (algo_name, digest, output_bits) = detect_algo(algo_name);

    // TODO: This is not supported by GNU. It is added here so we can use cksum
    // as a base for the specialized utils, but it should ultimately be hidden
    // on cksum itself.
    let binary = if matches.get_flag(options::BINARY) {
        true
    } else if matches.get_flag(options::TEXT) {
        false
    } else {
        BINARY_FLAG_DEFAULT
    };

    let check = matches.get_flag(options::CHECK);
    let tag = matches.get_flag(options::TAG) || !matches.get_flag(options::UNTAGGED);
    let status = matches.get_flag(options::STATUS);
    let quiet = matches.get_flag(options::QUIET) || status;
    let strict = matches.get_flag(options::STRICT);
    let warn = matches.get_flag(options::WARN) && !status;
    let zero = matches.get_flag(options::ZERO);

    let opts = Options {
        algo_name,
        digest,
        output_bits,
        binary,
        check,
        tag,
        status,
        quiet,
        strict,
        warn,
        _zero: zero,
    };

    match matches.get_many::<String>(options::FILE) {
        Some(files) => cksum(opts, files.map(OsStr::new))?,
        None => cksum(opts, iter::once(OsStr::new("-")))?,
    };

    Ok(())
}

/// The arguments to md5sum and similar utilities.
///
/// GNU documents this as md5sum-style, so that naming makes sense.
pub fn common_args() -> Vec<Arg> {
    #[cfg(windows)]
    const BINARY_HELP: &str = "read in binary mode (default)";
    #[cfg(not(windows))]
    const BINARY_HELP: &str = "read in binary mode";
    #[cfg(windows)]
    const TEXT_HELP: &str = "read in text mode";
    #[cfg(not(windows))]
    const TEXT_HELP: &str = "read in text mode (default)";

    vec![
        Arg::new(options::BINARY)
            .short('b')
            .long("binary")
            .help(BINARY_HELP)
            .action(ArgAction::SetTrue),
        Arg::new(options::CHECK)
            .short('c')
            .long("check")
            .help("read hashsums from the FILEs and check them")
            .action(ArgAction::SetTrue),
        // TODO: --ignore-missing
        Arg::new(options::QUIET)
            .short('q')
            .long("quiet")
            .help("don't print OK for each successfully verified file")
            .action(ArgAction::SetTrue),
        Arg::new(options::STATUS)
            .short('s')
            .long("status")
            .help("don't output anything, status code shows success")
            .action(ArgAction::SetTrue),
        Arg::new(options::TAG)
            .long("tag")
            .help("create a BSD-style checksum")
            .action(ArgAction::SetTrue),
        Arg::new(options::TEXT)
            .short('t')
            .long("text")
            .help(TEXT_HELP)
            .conflicts_with("binary")
            .action(ArgAction::SetTrue),
        Arg::new(options::WARN)
            .short('w')
            .long("warn")
            .help("warn about improperly formatted checksum lines")
            .action(ArgAction::SetTrue),
        Arg::new(options::STRICT)
            .long("strict")
            .help("exit non-zero for improperly formatted checksum lines")
            .action(ArgAction::SetTrue),
        Arg::new(options::ZERO)
            .short('z')
            .long("zero")
            .help("end each output line with NUL, not newline")
            .action(ArgAction::SetTrue),
    ]
}

/// b2sum-style args
///
/// Adds a length argument for the number of bits.
pub fn length_arg() -> Arg {
    Arg::new(options::LENGTH)
        .short('l')
        .long("length")
        .help(
            "digest length in bits; \
                must not exceed the max for the blake2 algorithm (512) and must be a multiple of 8",
        )
        .value_name("BITS")
        .value_parser(parse_bit_num)
}

// TODO: return custom error type
fn parse_bit_num(arg: &str) -> Result<usize, ParseIntError> {
    arg.parse()
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
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
                    algorithm::SYSV,
                    algorithm::BSD,
                    algorithm::CRC,
                    algorithm::MD5,
                    algorithm::SHA1,
                    algorithm::SHA224,
                    algorithm::SHA256,
                    algorithm::SHA384,
                    algorithm::SHA512,
                    algorithm::BLAKE2B,
                    algorithm::SM3,
                ]),
        )
        .arg(
            Arg::new(options::UNTAGGED)
                .long(options::UNTAGGED)
                .help("create a reversed style checksum, without digest type")
                .action(ArgAction::SetTrue)
                .overrides_with(options::TAG),
        )
        .args(common_args())
        .arg(length_arg())
        .after_help(AFTER_HELP)
}
