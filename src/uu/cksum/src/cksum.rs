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

fn detect_algo(program: &str) -> (&'static str, Box<dyn Digest + 'static>, usize) {
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
            Box::new(Blake2b::new()) as Box<dyn Digest>,
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

mod options {
    pub const ALGORITHM: &str = "algorithm";
    pub const FILE: &str = "file";
    pub const UNTAGGED: &str = "untagged";
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args = args.collect_ignore();

    let matches = uu_app().try_get_matches_from(args)?;

    let algo_name: &str = match matches.get_one::<String>(options::ALGORITHM) {
        Some(v) => v,
        None => ALGORITHM_OPTIONS_CRC,
    };

    let (name, algo, bits) = detect_algo(algo_name);
    let opts = Options {
        algo_name: name,
        digest: algo,
        output_bits: bits,
        untagged: matches.get_flag(options::UNTAGGED),
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
        .after_help(AFTER_HELP)
}
