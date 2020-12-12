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
extern crate blake2_rfc;
extern crate hex;
extern crate md5;
extern crate regex;
extern crate regex_syntax;
extern crate sha1;
extern crate sha2;
extern crate sha3;

#[macro_use]
extern crate uucore;

mod digest;

use self::digest::Digest;

use blake2_rfc::blake2b::Blake2b;
use clap::{App, Arg, ArgMatches};
use hex::ToHex;
use md5::Context as Md5;
use regex::Regex;
use sha1::Sha1;
use sha2::{Sha224, Sha256, Sha384, Sha512};
use sha3::{Sha3_224, Sha3_256, Sha3_384, Sha3_512, Shake128, Shake256};
use std::cmp::Ordering;
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::{self, stdin, BufRead, BufReader, Read};
use std::iter;
use std::num::ParseIntError;
use std::path::Path;

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
    #[allow(clippy::match_like_matches_macro)]
    // `matches!(...)` macro not stabilized until rust v1.42
    match program {
        "md5sum" | "sha1sum" | "sha224sum" | "sha256sum" | "sha384sum" | "sha512sum"
        | "sha3sum" | "sha3-224sum" | "sha3-256sum" | "sha3-384sum" | "sha3-512sum"
        | "shake128sum" | "shake256sum" | "b2sum" => true,
        _ => false,
    }
}

#[allow(clippy::cognitive_complexity)]
fn detect_algo<'a>(
    program: &str,
    matches: &ArgMatches<'a>,
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
        "b2sum" => ("BLAKE2", Box::new(Blake2b::new(64)) as Box<dyn Digest>, 512),
        "sha3sum" => match matches.value_of("bits") {
            Some(bits_str) => match usize::from_str_radix(&bits_str, 10) {
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
            Some(bits_str) => match usize::from_str_radix(&bits_str, 10) {
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
            Some(bits_str) => match usize::from_str_radix(&bits_str, 10) {
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
                    output_bits = bits
                };
                if matches.is_present("md5") {
                    set_or_crash("MD5", Box::new(Md5::new()), 128)
                }
                if matches.is_present("sha1") {
                    set_or_crash("SHA1", Box::new(Sha1::new()), 160)
                }
                if matches.is_present("sha224") {
                    set_or_crash("SHA224", Box::new(Sha224::new()), 224)
                }
                if matches.is_present("sha256") {
                    set_or_crash("SHA256", Box::new(Sha256::new()), 256)
                }
                if matches.is_present("sha384") {
                    set_or_crash("SHA384", Box::new(Sha384::new()), 384)
                }
                if matches.is_present("sha512") {
                    set_or_crash("SHA512", Box::new(Sha512::new()), 512)
                }
                if matches.is_present("b2sum") {
                    set_or_crash("BLAKE2", Box::new(Blake2b::new(64)), 512)
                }
                if matches.is_present("sha3") {
                    match matches.value_of("bits") {
                        Some(bits_str) => match usize::from_str_radix(&bits_str, 10) {
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
                    set_or_crash("SHA3-224", Box::new(Sha3_224::new()), 224)
                }
                if matches.is_present("sha3-256") {
                    set_or_crash("SHA3-256", Box::new(Sha3_256::new()), 256)
                }
                if matches.is_present("sha3-384") {
                    set_or_crash("SHA3-384", Box::new(Sha3_384::new()), 384)
                }
                if matches.is_present("sha3-512") {
                    set_or_crash("SHA3-512", Box::new(Sha3_512::new()), 512)
                }
                if matches.is_present("shake128") {
                    match matches.value_of("bits") {
                        Some(bits_str) => match usize::from_str_radix(&bits_str, 10) {
                            Ok(bits) => set_or_crash("SHAKE128", Box::new(Shake128::new()), bits),
                            Err(err) => crash!(1, "{}", err),
                        },
                        None => crash!(1, "--bits required for SHAKE-128"),
                    }
                }
                if matches.is_present("shake256") {
                    match matches.value_of("bits") {
                        Some(bits_str) => match usize::from_str_radix(&bits_str, 10) {
                            Ok(bits) => set_or_crash("SHAKE256", Box::new(Shake256::new()), bits),
                            Err(err) => crash!(1, "{}", err),
                        },
                        None => crash!(1, "--bits required for SHAKE-256"),
                    }
                }
            }
            if alg.is_none() {
                crash!(1, "You must specify hash algorithm!")
            };
            (name, alg.unwrap(), output_bits)
        }
    }
}

// TODO: return custom error type
fn parse_bit_num(arg: &str) -> Result<usize, ParseIntError> {
    usize::from_str_radix(arg, 10)
}

fn is_valid_bit_num(arg: String) -> Result<(), String> {
    parse_bit_num(&arg)
        .map(|_| ())
        .map_err(|e| format!("{}", e))
}

pub fn uumain(mut args: impl uucore::Args) -> i32 {
    // if there is no program name for some reason, default to "hashsum"
    let program = args.next().unwrap_or_else(|| OsString::from(NAME));
    let binary_name = Path::new(&program)
        .file_name()
        .unwrap_or_else(|| OsStr::new(NAME))
        .to_string_lossy();

    let args = iter::once(program.clone()).chain(args);

    // Default binary in Windows, text mode otherwise
    let binary_flag_default = cfg!(windows);

    let binary_help = format!(
        "read in binary mode{}",
        if binary_flag_default {
            " (default)"
        } else {
            ""
        }
    );

    let text_help = format!(
        "read in text mode{}",
        if binary_flag_default {
            ""
        } else {
            " (default)"
        }
    );

    let mut app = App::new(executable!())
        .version(crate_version!())
        .about("Compute and check message digests.")
        .arg(
            Arg::with_name("binary")
                .short("b")
                .long("binary")
                .help(&binary_help),
        )
        .arg(
            Arg::with_name("check")
                .short("c")
                .long("check")
                .help("read hashsums from the FILEs and check them"),
        )
        .arg(
            Arg::with_name("tag")
                .long("tag")
                .help("create a BSD-style checksum"),
        )
        .arg(
            Arg::with_name("text")
                .short("t")
                .long("text")
                .help(&text_help)
                .conflicts_with("binary"),
        )
        .arg(
            Arg::with_name("quiet")
                .short("q")
                .long("quiet")
                .help("don't print OK for each successfully verified file"),
        )
        .arg(
            Arg::with_name("status")
                .short("s")
                .long("status")
                .help("don't output anything, status code shows success"),
        )
        .arg(
            Arg::with_name("strict")
                .long("strict")
                .help("exit non-zero for improperly formatted checksum lines"),
        )
        .arg(
            Arg::with_name("warn")
                .short("w")
                .long("warn")
                .help("warn about improperly formatted checksum lines"),
        )
        // Needed for variable-length output sums (e.g. SHAKE)
        .arg(
            Arg::with_name("bits")
                .long("bits")
                .help("set the size of the output (only for SHAKE)")
                .takes_value(true)
                .value_name("BITS")
                // XXX: should we actually use validators?  they're not particularly efficient
                .validator(is_valid_bit_num),
        )
        .arg(
            Arg::with_name("FILE")
                .index(1)
                .multiple(true)
                .value_name("FILE"),
        );

    if !is_custom_binary(&binary_name) {
        let algos = &[
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
        ];

        for (name, desc) in algos {
            app = app.arg(Arg::with_name(name).long(name).help(desc));
        }
    }

    // FIXME: this should use get_matches_from_safe() and crash!(), but at the moment that just
    //        causes "error: " to be printed twice (once from crash!() and once from clap).  With
    //        the current setup, the name of the utility is not printed, but I think this is at
    //        least somewhat better from a user's perspective.
    let matches = app.get_matches_from(args);

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

    let res = match matches.values_of_os("FILE") {
        Some(files) => hashsum(opts, files),
        None => hashsum(opts, iter::once(OsStr::new("-"))),
    };

    match res {
        Ok(()) => 0,
        Err(e) => e,
    }
}

#[allow(clippy::cognitive_complexity)]
fn hashsum<'a, I>(mut options: Options, files: I) -> Result<(), i32>
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
            file_buf = safe_unwrap!(File::open(filename));
            Box::new(file_buf) as Box<dyn Read>
        });
        if options.check {
            // Set up Regexes for line validation and parsing
            let bytes = options.digest.output_bits() / 4;
            let gnu_re = safe_unwrap!(Regex::new(&format!(
                r"^(?P<digest>[a-fA-F0-9]{{{}}}) (?P<binary>[ \*])(?P<fileName>.*)",
                bytes
            )));
            let bsd_re = safe_unwrap!(Regex::new(&format!(
                r"^{algorithm} \((?P<fileName>.*)\) = (?P<digest>[a-fA-F0-9]{{{digest_size}}})",
                algorithm = options.algoname,
                digest_size = bytes
            )));

            let buffer = file;
            for (i, line) in buffer.lines().enumerate() {
                let line = safe_unwrap!(line);
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
                                return Err(1);
                            }
                            if options.warn {
                                show_warning!(
                                    "{}: {}: improperly formatted {} checksum line",
                                    filename.display(),
                                    i + 1,
                                    options.algoname
                                );
                            }
                            continue;
                        }
                    },
                };
                let f = safe_unwrap!(File::open(ck_filename));
                let mut ckf = BufReader::new(Box::new(f) as Box<dyn Read>);
                let real_sum = safe_unwrap!(digest_reader(
                    &mut *options.digest,
                    &mut ckf,
                    binary_check,
                    options.output_bits
                ))
                .to_ascii_lowercase();
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
            let sum = safe_unwrap!(digest_reader(
                &mut *options.digest,
                &mut file,
                options.binary,
                options.output_bits
            ));
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

fn digest_reader<'a, T: Read>(
    digest: &mut (dyn Digest + 'a),
    reader: &mut BufReader<T>,
    binary: bool,
    output_bits: usize,
) -> io::Result<String> {
    digest.reset();

    // Digest file, do not hold too much in memory at any given moment
    let windows = cfg!(windows);
    let mut buffer = Vec::with_capacity(524_288);
    let mut vec = Vec::with_capacity(524_288);
    let mut looking_for_newline = false;
    loop {
        match reader.read_to_end(&mut buffer) {
            Ok(0) => {
                break;
            }
            Ok(nread) => {
                if windows && !binary {
                    // Windows text mode returns '\n' when reading '\r\n'
                    for &b in buffer.iter().take(nread) {
                        if looking_for_newline {
                            if b != b'\n' {
                                vec.push(b'\r');
                            }
                            if b != b'\r' {
                                vec.push(b);
                                looking_for_newline = false;
                            }
                        } else if b != b'\r' {
                            vec.push(b);
                        } else {
                            looking_for_newline = true;
                        }
                    }
                    digest.input(&vec);
                    vec.clear();
                } else {
                    digest.input(&buffer[..nread]);
                }
            }
            Err(e) => return Err(e),
        }
    }
    if windows && looking_for_newline {
        vec.push(b'\r');
        digest.input(&vec);
    }

    if digest.output_bits() > 0 {
        Ok(digest.result_str())
    } else {
        // Assume it's SHAKE.  result_str() doesn't work with shake (as of 8/30/2016)
        let mut bytes = Vec::new();
        bytes.resize((output_bits + 7) / 8, 0);
        digest.result(&mut bytes);
        Ok(bytes.to_hex())
    }
}
