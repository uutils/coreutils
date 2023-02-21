// This file is part of the uutils coreutils package.
//
// (c) Michael Gehring <mg@ebfe.org>
//
//  For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) fname, algo
use clap::{crate_version, Arg, Command};
use hex::encode;
use std::ffi::OsStr;
use std::fs::File;
use std::io::{self, stdin, BufReader, Read};
use std::iter;
use std::path::Path;
use uucore::{
    error::{FromIo, UResult},
    format_usage,
    sum::{
        div_ceil, Blake2b, Digest, DigestWriter, Md5, Sha1, Sha224, Sha256, Sha384, Sha512, Sm3,
        BSD, CRC, SYSV,
    },
};

const USAGE: &str = "{} [OPTIONS] [FILE]...";
const ABOUT: &str = "Print CRC and size for each file";

fn detect_algo(program: &str) -> (&'static str, Box<dyn Digest + 'static>, usize) {
    match program {
        "sysv" => ("SYSV", Box::new(SYSV::new()) as Box<dyn Digest>, 512),
        "bsd" => ("BSD", Box::new(BSD::new()) as Box<dyn Digest>, 1024),
        "crc" => ("CRC", Box::new(CRC::new()) as Box<dyn Digest>, 256),
        "md5" => ("MD5", Box::new(Md5::new()) as Box<dyn Digest>, 128),
        "sha1" => ("SHA1", Box::new(Sha1::new()) as Box<dyn Digest>, 160),
        "sha224" => ("SHA224", Box::new(Sha224::new()) as Box<dyn Digest>, 224),
        "sha256" => ("SHA256", Box::new(Sha256::new()) as Box<dyn Digest>, 256),
        "sha384" => ("SHA384", Box::new(Sha384::new()) as Box<dyn Digest>, 384),
        "sha512" => ("SHA512", Box::new(Sha512::new()) as Box<dyn Digest>, 512),
        "blake2b" => ("BLAKE2", Box::new(Blake2b::new()) as Box<dyn Digest>, 512),
        "sm3" => ("SM3", Box::new(Sm3::new()) as Box<dyn Digest>, 512),
        _ => panic!("unknown algorithm"),
    }
}

struct Options {
    algo_name: &'static str,
    digest: Box<dyn Digest + 'static>,
    output_bits: usize,
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
            ("SYSV", true) => println!(
                "{} {}",
                sum.parse::<u16>().unwrap(),
                div_ceil(sz, options.output_bits)
            ),
            ("SYSV", false) => println!(
                "{} {} {}",
                sum.parse::<u16>().unwrap(),
                div_ceil(sz, options.output_bits),
                filename.display()
            ),
            ("BSD", true) => println!(
                "{:0bsd_width$} {:bsd_width$}",
                sum.parse::<u16>().unwrap(),
                div_ceil(sz, options.output_bits)
            ),
            ("BSD", false) => println!(
                "{:0bsd_width$} {:bsd_width$} {}",
                sum.parse::<u16>().unwrap(),
                div_ceil(sz, options.output_bits),
                filename.display()
            ),
            (_, true) => println!("{sum} {sz}"),
            (_, false) => println!("{sum} {sz} {}", filename.display()),
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
        let mut bytes = Vec::new();
        bytes.resize((output_bits + 7) / 8, 0);
        digest.hash_finalize(&mut bytes);
        Ok((encode(bytes), output_size))
    }
}

mod options {
    pub static FILE: &str = "file";
    pub static ALGORITHM: &str = "algorithm";
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args = args.collect_ignore();

    let matches = uu_app().try_get_matches_from(args)?;

    let algo_name: &str = match matches.get_one::<String>(options::ALGORITHM) {
        Some(v) => v,
        None => "crc",
    };

    let (name, algo, bits) = detect_algo(algo_name);
    let opts = Options {
        algo_name: name,
        digest: algo,
        output_bits: bits,
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
                .value_name("ALGORITHM"),
        )
}
