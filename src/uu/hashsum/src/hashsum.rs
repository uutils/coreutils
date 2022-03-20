//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Alex Lyon <arcterus@mail.com>
//  * (c) Vsevolod Velichko <torkvemada@sorokdva.net>
//  * (c) Gil Cottle <gcottle@redtown.org>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) algo, algoname, regexes, nread

#[macro_use]
extern crate clap;

#[macro_use]
extern crate uucore;

mod digest;

use self::digest::Digest;
use self::digest::DigestWriter;

use clap::{Arg, ArgMatches, Command};
use hex::encode;
use md5::Md5;
use regex::Regex;
use sha1::Sha1;
use sha2::{Sha224, Sha256, Sha384, Sha512};
use sha3::{Sha3_224, Sha3_256, Sha3_384, Sha3_512, Shake128, Shake256};
use std::cmp::Ordering;
use std::error::Error;
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::{self, stdin, BufRead, BufReader, Read};
use std::iter;
use std::num::ParseIntError;
use std::path::Path;
use uucore::display::Quotable;
use uucore::error::{FromIo, UError, UResult};

const NAME: &str = "hashsum";

struct Options {
    algoname: &'static str,
    digest: Box<dyn Digest + 'static>,
    binary: bool,
    check: bool,
    tag: bool,
    status: bool,
    quiet: bool,
    strict: bool,
    warn: bool,
    output_bits: usize,
}

fn is_custom_binary(program: &str) -> bool {
    matches!(
        program,
        "md5sum"
            | "sha1sum"
            | "sha224sum"
            | "sha256sum"
            | "sha384sum"
            | "sha512sum"
            | "sha3sum"
            | "sha3-224sum"
            | "sha3-256sum"
            | "sha3-384sum"
            | "sha3-512sum"
            | "shake128sum"
            | "shake256sum"
            | "b2sum"
            | "b3sum"
    )
}

#[allow(clippy::cognitive_complexity)]
fn detect_algo(
    program: &str,
    matches: &ArgMatches,
) -> (&'static str, Box<dyn Digest + 'static>, usize) {
    let mut alg: Option<Box<dyn Digest>> = None;
    let mut name: &'static str = "";
    let mut output_bits = 0;
    match program {
        "md5sum" => ("MD5", Box::new(Md5::new()) as Box<dyn Digest>, 128),
        "sha1sum" => ("SHA1", Box::new(Sha1::new()) as Box<dyn Digest>, 160),
        "sha224sum" => ("SHA224", Box::new(Sha224::new()) as Box<dyn Digest>, 224),
        "sha256sum" => ("SHA256", Box::new(Sha256::new()) as Box<dyn Digest>, 256),
        "sha384sum" => ("SHA384", Box::new(Sha384::new()) as Box<dyn Digest>, 384),
        "sha512sum" => ("SHA512", Box::new(Sha512::new()) as Box<dyn Digest>, 512),
        "b2sum" => (
            "BLAKE2",
            Box::new(blake2b_simd::State::new()) as Box<dyn Digest>,
            512,
        ),
        "b3sum" => (
            "BLAKE3",
            Box::new(blake3::Hasher::new()) as Box<dyn Digest>,
            256,
        ),
        "sha3sum" => match matches.value_of("bits") {
            Some(bits_str) => match (bits_str).parse::<usize>() {
                Ok(224) => (
                    "SHA3-224",
                    Box::new(Sha3_224::new()) as Box<dyn Digest>,
                    224,
                ),
                Ok(256) => (
                    "SHA3-256",
                    Box::new(Sha3_256::new()) as Box<dyn Digest>,
                    256,
                ),
                Ok(384) => (
                    "SHA3-384",
                    Box::new(Sha3_384::new()) as Box<dyn Digest>,
                    384,
                ),
                Ok(512) => (
                    "SHA3-512",
                    Box::new(Sha3_512::new()) as Box<dyn Digest>,
                    512,
                ),
                Ok(_) => crash!(
                    1,
                    "Invalid output size for SHA3 (expected 224, 256, 384, or 512)"
                ),
                Err(err) => crash!(1, "{}", err),
            },
            None => crash!(1, "--bits required for SHA3"),
        },
        "sha3-224sum" => (
            "SHA3-224",
            Box::new(Sha3_224::new()) as Box<dyn Digest>,
            224,
        ),
        "sha3-256sum" => (
            "SHA3-256",
            Box::new(Sha3_256::new()) as Box<dyn Digest>,
            256,
        ),
        "sha3-384sum" => (
            "SHA3-384",
            Box::new(Sha3_384::new()) as Box<dyn Digest>,
            384,
        ),
        "sha3-512sum" => (
            "SHA3-512",
            Box::new(Sha3_512::new()) as Box<dyn Digest>,
            512,
        ),
        "shake128sum" => match matches.value_of("bits") {
            Some(bits_str) => match (bits_str).parse::<usize>() {
                Ok(bits) => (
                    "SHAKE128",
                    Box::new(Shake128::new()) as Box<dyn Digest>,
                    bits,
                ),
                Err(err) => crash!(1, "{}", err),
            },
            None => crash!(1, "--bits required for SHAKE-128"),
        },
        "shake256sum" => match matches.value_of("bits") {
            Some(bits_str) => match (bits_str).parse::<usize>() {
                Ok(bits) => (
                    "SHAKE256",
                    Box::new(Shake256::new()) as Box<dyn Digest>,
                    bits,
                ),
                Err(err) => crash!(1, "{}", err),
            },
            None => crash!(1, "--bits required for SHAKE-256"),
        },
        _ => {
            {
                let mut set_or_crash = |n, val, bits| {
                    if alg.is_some() {
                        crash!(1, "You cannot combine multiple hash algorithms!")
                    };
                    name = n;
                    alg = Some(val);
                    output_bits = bits;
                };
                if matches.is_present("md5") {
                    set_or_crash("MD5", Box::new(Md5::new()), 128);
                }
                if matches.is_present("sha1") {
                    set_or_crash("SHA1", Box::new(Sha1::new()), 160);
                }
                if matches.is_present("sha224") {
                    set_or_crash("SHA224", Box::new(Sha224::new()), 224);
                }
                if matches.is_present("sha256") {
                    set_or_crash("SHA256", Box::new(Sha256::new()), 256);
                }
                if matches.is_present("sha384") {
                    set_or_crash("SHA384", Box::new(Sha384::new()), 384);
                }
                if matches.is_present("sha512") {
                    set_or_crash("SHA512", Box::new(Sha512::new()), 512);
                }
                if matches.is_present("b2sum") {
                    set_or_crash("BLAKE2", Box::new(blake2b_simd::State::new()), 512);
                }
                if matches.is_present("b3sum") {
                    set_or_crash("BLAKE3", Box::new(blake3::Hasher::new()), 256);
                }
                if matches.is_present("sha3") {
                    match matches.value_of("bits") {
                        Some(bits_str) => match (bits_str).parse::<usize>() {
                            Ok(224) => set_or_crash(
                                "SHA3-224",
                                Box::new(Sha3_224::new()) as Box<dyn Digest>,
                                224,
                            ),
                            Ok(256) => set_or_crash(
                                "SHA3-256",
                                Box::new(Sha3_256::new()) as Box<dyn Digest>,
                                256,
                            ),
                            Ok(384) => set_or_crash(
                                "SHA3-384",
                                Box::new(Sha3_384::new()) as Box<dyn Digest>,
                                384,
                            ),
                            Ok(512) => set_or_crash(
                                "SHA3-512",
                                Box::new(Sha3_512::new()) as Box<dyn Digest>,
                                512,
                            ),
                            Ok(_) => crash!(
                                1,
                                "Invalid output size for SHA3 (expected 224, 256, 384, or 512)"
                            ),
                            Err(err) => crash!(1, "{}", err),
                        },
                        None => crash!(1, "--bits required for SHA3"),
                    }
                }
                if matches.is_present("sha3-224") {
                    set_or_crash("SHA3-224", Box::new(Sha3_224::new()), 224);
                }
                if matches.is_present("sha3-256") {
                    set_or_crash("SHA3-256", Box::new(Sha3_256::new()), 256);
                }
                if matches.is_present("sha3-384") {
                    set_or_crash("SHA3-384", Box::new(Sha3_384::new()), 384);
                }
                if matches.is_present("sha3-512") {
                    set_or_crash("SHA3-512", Box::new(Sha3_512::new()), 512);
                }
                if matches.is_present("shake128") {
                    match matches.value_of("bits") {
                        Some(bits_str) => match (bits_str).parse::<usize>() {
                            Ok(bits) => set_or_crash("SHAKE128", Box::new(Shake128::new()), bits),
                            Err(err) => crash!(1, "{}", err),
                        },
                        None => crash!(1, "--bits required for SHAKE-128"),
                    }
                }
                if matches.is_present("shake256") {
                    match matches.value_of("bits") {
                        Some(bits_str) => match (bits_str).parse::<usize>() {
                            Ok(bits) => set_or_crash("SHAKE256", Box::new(Shake256::new()), bits),
                            Err(err) => crash!(1, "{}", err),
                        },
                        None => crash!(1, "--bits required for SHAKE-256"),
                    }
                }
            }
            let alg = alg.unwrap_or_else(|| crash!(1, "You must specify hash algorithm!"));
            (name, alg, output_bits)
        }
    }
}

// TODO: return custom error type
fn parse_bit_num(arg: &str) -> Result<usize, ParseIntError> {
    arg.parse()
}

fn is_valid_bit_num(arg: &str) -> Result<(), String> {
    parse_bit_num(arg).map(|_| ()).map_err(|e| format!("{}", e))
}

#[uucore::main]
pub fn uumain(mut args: impl uucore::Args) -> UResult<()> {
    // if there is no program name for some reason, default to "hashsum"
    let program = args.next().unwrap_or_else(|| OsString::from(NAME));
    let binary_name = Path::new(&program)
        .file_name()
        .unwrap_or_else(|| OsStr::new(NAME))
        .to_string_lossy();

    let args = iter::once(program.clone()).chain(args);

    // Default binary in Windows, text mode otherwise
    let binary_flag_default = cfg!(windows);

    let command = uu_app(&binary_name);

    // FIXME: this should use try_get_matches_from() and crash!(), but at the moment that just
    //        causes "error: " to be printed twice (once from crash!() and once from clap).  With
    //        the current setup, the name of the utility is not printed, but I think this is at
    //        least somewhat better from a user's perspective.
    let matches = command.get_matches_from(args);

    let (name, algo, bits) = detect_algo(&binary_name, &matches);

    let binary = if matches.is_present("binary") {
        true
    } else if matches.is_present("text") {
        false
    } else {
        binary_flag_default
    };
    let check = matches.is_present("check");
    let tag = matches.is_present("tag");
    let status = matches.is_present("status");
    let quiet = matches.is_present("quiet") || status;
    let strict = matches.is_present("strict");
    let warn = matches.is_present("warn") && !status;

    let opts = Options {
        algoname: name,
        digest: algo,
        output_bits: bits,
        binary,
        check,
        tag,
        status,
        quiet,
        strict,
        warn,
    };

    match matches.values_of_os("FILE") {
        Some(files) => hashsum(opts, files),
        None => hashsum(opts, iter::once(OsStr::new("-"))),
    }
}

pub fn uu_app_common<'a>() -> Command<'a> {
    #[cfg(windows)]
    const BINARY_HELP: &str = "read in binary mode (default)";
    #[cfg(not(windows))]
    const BINARY_HELP: &str = "read in binary mode";
    #[cfg(windows)]
    const TEXT_HELP: &str = "read in text mode";
    #[cfg(not(windows))]
    const TEXT_HELP: &str = "read in text mode (default)";
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about("Compute and check message digests.")
        .infer_long_args(true)
        .arg(
            Arg::new("binary")
                .short('b')
                .long("binary")
                .help(BINARY_HELP),
        )
        .arg(
            Arg::new("check")
                .short('c')
                .long("check")
                .help("read hashsums from the FILEs and check them"),
        )
        .arg(
            Arg::new("tag")
                .long("tag")
                .help("create a BSD-style checksum"),
        )
        .arg(
            Arg::new("text")
                .short('t')
                .long("text")
                .help(TEXT_HELP)
                .conflicts_with("binary"),
        )
        .arg(
            Arg::new("quiet")
                .short('q')
                .long("quiet")
                .help("don't print OK for each successfully verified file"),
        )
        .arg(
            Arg::new("status")
                .short('s')
                .long("status")
                .help("don't output anything, status code shows success"),
        )
        .arg(
            Arg::new("strict")
                .long("strict")
                .help("exit non-zero for improperly formatted checksum lines"),
        )
        .arg(
            Arg::new("warn")
                .short('w')
                .long("warn")
                .help("warn about improperly formatted checksum lines"),
        )
        // Needed for variable-length output sums (e.g. SHAKE)
        .arg(
            Arg::new("bits")
                .long("bits")
                .help("set the size of the output (only for SHAKE)")
                .takes_value(true)
                .value_name("BITS")
                // XXX: should we actually use validators?  they're not particularly efficient
                .validator(is_valid_bit_num),
        )
        .arg(
            Arg::new("FILE")
                .index(1)
                .multiple_occurrences(true)
                .value_name("FILE")
                .allow_invalid_utf8(true),
        )
}

pub fn uu_app_custom<'a>() -> Command<'a> {
    let mut command = uu_app_common();
    let algorithms = &[
        ("md5", "work with MD5"),
        ("sha1", "work with SHA1"),
        ("sha224", "work with SHA224"),
        ("sha256", "work with SHA256"),
        ("sha384", "work with SHA384"),
        ("sha512", "work with SHA512"),
        ("sha3", "work with SHA3"),
        ("sha3-224", "work with SHA3-224"),
        ("sha3-256", "work with SHA3-256"),
        ("sha3-384", "work with SHA3-384"),
        ("sha3-512", "work with SHA3-512"),
        (
            "shake128",
            "work with SHAKE128 using BITS for the output size",
        ),
        (
            "shake256",
            "work with SHAKE256 using BITS for the output size",
        ),
        ("b2sum", "work with BLAKE2"),
        ("b3sum", "work with BLAKE3"),
    ];

    for (name, desc) in algorithms {
        command = command.arg(Arg::new(*name).long(name).help(*desc));
    }
    command
}

// hashsum is handled differently in build.rs, therefore this is not the same
// as in other utilities.
fn uu_app<'a>(binary_name: &str) -> Command<'a> {
    if !is_custom_binary(binary_name) {
        uu_app_custom()
    } else {
        uu_app_common()
    }
}

#[derive(Debug)]
enum HashsumError {
    InvalidRegex,
    InvalidFormat,
}

impl Error for HashsumError {}
impl UError for HashsumError {}

impl std::fmt::Display for HashsumError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            HashsumError::InvalidRegex => write!(f, "invalid regular expression"),
            HashsumError::InvalidFormat => Ok(()),
        }
    }
}

#[allow(clippy::cognitive_complexity)]
fn hashsum<'a, I>(mut options: Options, files: I) -> UResult<()>
where
    I: Iterator<Item = &'a OsStr>,
{
    let mut bad_format = 0;
    let mut failed = 0;
    let binary_marker = if options.binary { "*" } else { " " };
    for filename in files {
        let filename = Path::new(filename);

        let stdin_buf;
        let file_buf;
        let mut file = BufReader::new(if filename == OsStr::new("-") {
            stdin_buf = stdin();
            Box::new(stdin_buf) as Box<dyn Read>
        } else {
            file_buf =
                File::open(filename).map_err_context(|| "failed to open file".to_string())?;
            Box::new(file_buf) as Box<dyn Read>
        });
        if options.check {
            // Set up Regexes for line validation and parsing
            //
            // First, we compute the number of bytes we expect to be in
            // the digest string. If the algorithm has a variable number
            // of output bits, then we use the `+` modifier in the
            // regular expression, otherwise we use the `{n}` modifier,
            // where `n` is the number of bytes.
            let bytes = options.digest.output_bits() / 4;
            let modifier = if bytes > 0 {
                format!("{{{}}}", bytes)
            } else {
                "+".to_string()
            };
            let gnu_re = Regex::new(&format!(
                r"^(?P<digest>[a-fA-F0-9]{}) (?P<binary>[ \*])(?P<fileName>.*)",
                modifier,
            ))
            .map_err(|_| HashsumError::InvalidRegex)?;
            let bsd_re = Regex::new(&format!(
                r"^{algorithm} \((?P<fileName>.*)\) = (?P<digest>[a-fA-F0-9]{digest_size})",
                algorithm = options.algoname,
                digest_size = modifier,
            ))
            .map_err(|_| HashsumError::InvalidRegex)?;

            let buffer = file;
            for (i, maybe_line) in buffer.lines().enumerate() {
                let line = match maybe_line {
                    Ok(l) => l,
                    Err(e) => return Err(e.map_err_context(|| "failed to read file".to_string())),
                };
                let (ck_filename, sum, binary_check) = match gnu_re.captures(&line) {
                    Some(caps) => (
                        caps.name("fileName").unwrap().as_str(),
                        caps.name("digest").unwrap().as_str().to_ascii_lowercase(),
                        caps.name("binary").unwrap().as_str() == "*",
                    ),
                    None => match bsd_re.captures(&line) {
                        Some(caps) => (
                            caps.name("fileName").unwrap().as_str(),
                            caps.name("digest").unwrap().as_str().to_ascii_lowercase(),
                            true,
                        ),
                        None => {
                            bad_format += 1;
                            if options.strict {
                                return Err(HashsumError::InvalidFormat.into());
                            }
                            if options.warn {
                                show_warning!(
                                    "{}: {}: improperly formatted {} checksum line",
                                    filename.maybe_quote(),
                                    i + 1,
                                    options.algoname
                                );
                            }
                            continue;
                        }
                    },
                };
                let f = File::open(ck_filename)
                    .map_err_context(|| "failed to open file".to_string())?;
                let mut ckf = BufReader::new(Box::new(f) as Box<dyn Read>);
                let real_sum = digest_reader(
                    &mut options.digest,
                    &mut ckf,
                    binary_check,
                    options.output_bits,
                )
                .map_err_context(|| "failed to read input".to_string())?
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
                        println!("{}: OK", ck_filename);
                    }
                } else {
                    if !options.status {
                        println!("{}: FAILED", ck_filename);
                    }
                    failed += 1;
                }
            }
        } else {
            let sum = digest_reader(
                &mut options.digest,
                &mut file,
                options.binary,
                options.output_bits,
            )
            .map_err_context(|| "failed to read input".to_string())?;
            if options.tag {
                println!("{} ({}) = {}", options.algoname, filename.display(), sum);
            } else {
                println!("{} {}{}", sum, binary_marker, filename.display());
            }
        }
    }
    if !options.status {
        match bad_format.cmp(&1) {
            Ordering::Equal => show_warning!("{} line is improperly formatted", bad_format),
            Ordering::Greater => show_warning!("{} lines are improperly formatted", bad_format),
            _ => {}
        };
        if failed > 0 {
            show_warning!("{} computed checksum did NOT match", failed);
        }
    }

    Ok(())
}

fn digest_reader<T: Read>(
    digest: &mut Box<dyn Digest>,
    reader: &mut BufReader<T>,
    binary: bool,
    output_bits: usize,
) -> io::Result<String> {
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
    std::io::copy(reader, &mut digest_writer)?;
    digest_writer.finalize();

    if digest.output_bits() > 0 {
        Ok(digest.result_str())
    } else {
        // Assume it's SHAKE.  result_str() doesn't work with shake (as of 8/30/2016)
        let mut bytes = Vec::new();
        bytes.resize((output_bits + 7) / 8, 0);
        digest.result(&mut bytes);
        Ok(encode(bytes))
    }
}
