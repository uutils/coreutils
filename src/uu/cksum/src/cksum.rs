// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) fname, algo
use clap::{crate_version, Arg, ArgAction, Command};
use hex::encode;
use std::ffi::OsStr;
use std::fs::File;
use std::io::{self, stdin, BufReader, Read};
use std::iter;
use std::num::ParseIntError;
use std::path::Path;
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
    untagged: bool,
    output_bits: usize,

    // common
    binary: bool,
    check: bool,
    tag: bool,
    status: bool,
    quiet: bool,
    strict: bool,
    warn: bool,
    zero: bool,
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
    for filename in files {
        let filename = Path::new(filename);
        let stdin_buf;
        let file_buf;
        let not_file = filename == OsStr::new("-");
        let mut file = BufReader::new(if not_file {
            stdin_buf = stdin();
            Box::new(stdin_buf) as Box<dyn Read>
        } else if filename.is_dir() {
            Box::new(BufReader::new(io::empty())) as Box<dyn Read>
        } else {
            file_buf =
                File::open(filename).map_err_context(|| filename.to_str().unwrap().to_string())?;
            Box::new(file_buf) as Box<dyn Read>
        });
        let (sum, sz) = digest_read(&mut options.digest, &mut file, options.output_bits)
            .map_err_context(|| "failed to read input".to_string())?;

        // The BSD checksum output is 5 digit integer
        let bsd_width = 5;
        match (options.algo_name, not_file) {
            (algorithm::SYSV, true) => println!(
                "{} {}",
                sum.parse::<u16>().unwrap(),
                div_ceil(sz, options.output_bits)
            ),
            (algorithm::SYSV, false) => println!(
                "{} {} {}",
                sum.parse::<u16>().unwrap(),
                div_ceil(sz, options.output_bits),
                filename.display()
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
                filename.display()
            ),
            (algorithm::CRC, true) => println!("{sum} {sz}"),
            (algorithm::CRC, false) => println!("{sum} {sz} {}", filename.display()),
            (algorithm::BLAKE2B, _) if !options.untagged => {
                println!("BLAKE2b ({}) = {sum}", filename.display());
            }
            _ => {
                if options.untagged {
                    println!("{sum}  {}", filename.display());
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

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().try_get_matches_from(args)?;

    let algo_name: &str = match matches.get_one::<String>(options::ALGORITHM) {
        Some(v) => v,
        None => algorithm::CRC,
    };

    let (algo_name, digest, output_bits) = detect_algo(algo_name);

    let untagged = matches.get_flag(options::UNTAGGED);

    let binary = if matches.get_flag(options::BINARY) {
        true
    } else if matches.get_flag(options::TEXT) {
        false
    } else {
        BINARY_FLAG_DEFAULT
    };
    let check = matches.get_flag(options::CHECK);
    let tag = matches.get_flag(options::TAG);
    let status = matches.get_flag(options::STATUS);
    let quiet = matches.get_flag(options::QUIET) || status;
    let strict = matches.get_flag(options::STRICT);
    let warn = matches.get_flag(options::WARN) && !status;
    let zero = matches.get_flag(options::ZERO);

    let opts = Options {
        algo_name,
        digest,
        output_bits,
        untagged,

        binary,
        check,
        tag,
        status,
        quiet,
        strict,
        warn,
        zero,
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
                .action(ArgAction::SetTrue),
        )
        .args(common_args())
        .arg(length_arg())
        .after_help(AFTER_HELP)
}
