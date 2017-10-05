#![crate_name = "uu_hashsum"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Arcterus <arcterus@mail.com>
 * (c) Vsevolod Velichko <torkvemada@sorokdva.net>
 * (c) Gil Cottle <gcottle@redtown.org>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate crypto;
extern crate getopts;
extern crate regex_syntax;
extern crate regex;
extern crate rustc_serialize as serialize;

#[macro_use]
extern crate uucore;

use crypto::digest::Digest;
use crypto::md5::Md5;
use crypto::sha1::Sha1;
use crypto::sha2::{Sha224, Sha256, Sha384, Sha512};
use crypto::sha3::{Sha3, Sha3Mode};
use regex::Regex;
use serialize::hex::ToHex;
use std::ascii::AsciiExt;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read, stdin, Write};
use std::path::Path;

static NAME: &'static str = "hashsum";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

fn is_custom_binary(program: &str) -> bool {
    match program {
        "md5sum" | "sha1sum"
            | "sha224sum" | "sha256sum"
            | "sha384sum" | "sha512sum"
            | "sha3sum" | "sha3-224sum"
            | "sha3-256sum" | "sha3-384sum"
            | "sha3-512sum" | "shake128sum"
            | "shake256sum" => true,
        _ => false
    }
}

fn detect_algo(program: &str, matches: &getopts::Matches) -> (&'static str, Box<Digest+'static>, usize) {
    let mut alg: Option<Box<Digest>> = None;
    let mut name: &'static str = "";
    let mut output_bits = 0;
    match program {
        "md5sum" => ("MD5", Box::new(Md5::new()) as Box<Digest>, 128),
        "sha1sum" => ("SHA1", Box::new(Sha1::new()) as Box<Digest>, 160),
        "sha224sum" => ("SHA224", Box::new(Sha224::new()) as Box<Digest>, 224),
        "sha256sum" => ("SHA256", Box::new(Sha256::new()) as Box<Digest>, 256),
        "sha384sum" => ("SHA384", Box::new(Sha384::new()) as Box<Digest>, 384),
        "sha512sum" => ("SHA512", Box::new(Sha512::new()) as Box<Digest>, 512),
        "sha3sum" => {
            match matches.opt_str("bits") {
                Some(bits_str) => match usize::from_str_radix(&bits_str, 10) {
                    Ok(224) => ("SHA3-224", Box::new(Sha3::new(Sha3Mode::Sha3_224)) as Box<Digest>, 224),
                    Ok(256) => ("SHA3-256", Box::new(Sha3::new(Sha3Mode::Sha3_256)) as Box<Digest>, 256),
                    Ok(384) => ("SHA3-384", Box::new(Sha3::new(Sha3Mode::Sha3_384)) as Box<Digest>, 384),
                    Ok(512) => ("SHA3-512", Box::new(Sha3::new(Sha3Mode::Sha3_512)) as Box<Digest>, 512),
                    Ok(_) => crash!(1, "Invalid output size for SHA3 (expected 224, 256, 384, or 512)"),
                    Err(err) => crash!(1, "{}", err)
                },
                None => crash!(1, "--bits required for SHA3")
            }
        }
        "sha3-224sum" => ("SHA3-224", Box::new(Sha3::new(Sha3Mode::Sha3_224)) as Box<Digest>, 224),
        "sha3-256sum" => ("SHA3-256", Box::new(Sha3::new(Sha3Mode::Sha3_256)) as Box<Digest>, 256),
        "sha3-384sum" => ("SHA3-384", Box::new(Sha3::new(Sha3Mode::Sha3_384)) as Box<Digest>, 384),
        "sha3-512sum" => ("SHA3-512", Box::new(Sha3::new(Sha3Mode::Sha3_512)) as Box<Digest>, 512),
        "shake128sum" => {
            match matches.opt_str("bits") {
                Some(bits_str) => match usize::from_str_radix(&bits_str, 10) {
                    Ok(bits) => ("SHAKE128", Box::new(Sha3::new(Sha3Mode::Shake128)) as Box<Digest>, bits),
                    Err(err) => crash!(1, "{}", err)
                },
                None => crash!(1, "--bits required for SHAKE-128")
            }
        }
        "shake256sum" => {
            match matches.opt_str("bits") {
                Some(bits_str) => match usize::from_str_radix(&bits_str, 10) {
                    Ok(bits) => ("SHAKE256", Box::new(Sha3::new(Sha3Mode::Shake256)) as Box<Digest>, bits),
                    Err(err) => crash!(1, "{}", err)
                },
                None => crash!(1, "--bits required for SHAKE-256")
            }
        }
        _ => {
            {
                let mut set_or_crash = |n, val, bits| -> () {
                    if alg.is_some() { crash!(1, "You cannot combine multiple hash algorithms!") };
                    name = n;
                    alg = Some(val);
                    output_bits = bits
                };
                if matches.opt_present("md5") { set_or_crash("MD5", Box::new(Md5::new()), 128) }
                if matches.opt_present("sha1") { set_or_crash("SHA1", Box::new(Sha1::new()), 160) }
                if matches.opt_present("sha224") { set_or_crash("SHA224", Box::new(Sha224::new()), 224) }
                if matches.opt_present("sha256") { set_or_crash("SHA256", Box::new(Sha256::new()), 256) }
                if matches.opt_present("sha384") { set_or_crash("SHA384", Box::new(Sha384::new()), 384) }
                if matches.opt_present("sha512") { set_or_crash("SHA512", Box::new(Sha512::new()), 512) }
                if matches.opt_present("sha3") {
                    match matches.opt_str("bits") {
                        Some(bits_str) => match usize::from_str_radix(&bits_str, 10) {
                            Ok(224) => set_or_crash("SHA3-224", Box::new(Sha3::new(Sha3Mode::Sha3_224)) as Box<Digest>, 224),
                            Ok(256) => set_or_crash("SHA3-256", Box::new(Sha3::new(Sha3Mode::Sha3_256)) as Box<Digest>, 256),
                            Ok(384) => set_or_crash("SHA3-384", Box::new(Sha3::new(Sha3Mode::Sha3_384)) as Box<Digest>, 384),
                            Ok(512) => set_or_crash("SHA3-512", Box::new(Sha3::new(Sha3Mode::Sha3_512)) as Box<Digest>, 512),
                            Ok(_) => crash!(1, "Invalid output size for SHA3 (expected 224, 256, 384, or 512)"),
                            Err(err) => crash!(1, "{}", err)
                        },
                        None => crash!(1, "--bits required for SHA3")
                    }
                }
                if matches.opt_present("sha3-224") { set_or_crash("SHA3-224", Box::new(Sha3::new(Sha3Mode::Sha3_224)), 224) }
                if matches.opt_present("sha3-256") { set_or_crash("SHA3-256", Box::new(Sha3::new(Sha3Mode::Sha3_256)), 256) }
                if matches.opt_present("sha3-384") { set_or_crash("SHA3-384", Box::new(Sha3::new(Sha3Mode::Sha3_384)), 384) }
                if matches.opt_present("sha3-512") { set_or_crash("SHA3-512", Box::new(Sha3::new(Sha3Mode::Sha3_512)), 512) }
                if matches.opt_present("shake128") {
                    match matches.opt_str("bits") {
                        Some(bits_str) => match usize::from_str_radix(&bits_str, 10) {
                            Ok(bits) => set_or_crash("SHAKE128", Box::new(Sha3::new(Sha3Mode::Shake128)), bits),
                            Err(err) => crash!(1, "{}", err)
                        },
                        None => crash!(1, "--bits required for SHAKE-128")
                    }
                }
                if matches.opt_present("shake256") {
                    match matches.opt_str("bits") {
                        Some(bits_str) => match usize::from_str_radix(&bits_str, 10) {
                            Ok(bits) => set_or_crash("SHAKE256", Box::new(Sha3::new(Sha3Mode::Shake256)), bits),
                            Err(err) => crash!(1, "{}", err)
                        },
                        None => crash!(1, "--bits required for SHAKE-256")
                    }
                }
            }
            if alg.is_none() { crash!(1, "You must specify hash algorithm!") };
            (name, alg.unwrap(), output_bits)
        }
    }
}

pub fn uumain(args: Vec<String>) -> i32 {
    let program = &args[0];
    let binary_name = Path::new(program).file_name().unwrap().to_str().unwrap();

    // Default binary in Windows, text mode otherwise
    let binary_flag_default = cfg!(windows);

    let mut opts = getopts::Options::new();
    opts.optflag("b", "binary", &format!("read in binary mode{}", if binary_flag_default { " (default)" } else { "" }));
    opts.optflag("c", "check", "read hashsums from the FILEs and check them");
    opts.optflag("", "tag", "create a BSD-style checksum");
    opts.optflag("t", "text", &format!("read in text mode{}", if binary_flag_default { "" } else { " (default)" }));
    opts.optflag("q", "quiet", "don't print OK for each successfully verified file");
    opts.optflag("s", "status", "don't output anything, status code shows success");
    opts.optflag("", "strict", "exit non-zero for improperly formatted checksum lines");
    opts.optflag("w", "warn", "warn about improperly formatted checksum lines");
    opts.optflag("h", "help", "display this help and exit");
    opts.optflag("V", "version", "output version information and exit");

    if !is_custom_binary(program) {
        opts.optflag("", "md5", "work with MD5");
        opts.optflag("", "sha1", "work with SHA1");
        opts.optflag("", "sha224", "work with SHA224");
        opts.optflag("", "sha256", "work with SHA256");
        opts.optflag("", "sha384", "work with SHA384");
        opts.optflag("", "sha512", "work with SHA512");
        opts.optflag("", "sha3", "work with SHA3");
        opts.optflag("", "sha3-224", "work with SHA3-224");
        opts.optflag("", "sha3-256", "work with SHA3-256");
        opts.optflag("", "sha3-384", "work with SHA3-384");
        opts.optflag("", "sha3-512", "work with SHA3-512");
        opts.optflag("", "shake128", "work with SHAKE128 using BITS for the output size");
        opts.optflag("", "shake256", "work with SHAKE256 using BITS for the output size");
    }

    // Needed for variable-length output sums (e.g. SHAKE)
    opts.optopt("", "bits", "set the size of the output (only for SHAKE)", "BITS");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => crash!(1, "{}", f)
    };

    if matches.opt_present("help") {
        usage(program, binary_name, &opts);
    } else if matches.opt_present("version") {
        version();
    } else {
        let (name, algo, bits) = detect_algo(binary_name, &matches);

        let binary_flag = matches.opt_present("binary");
        let text_flag = matches.opt_present("text");
        if binary_flag && text_flag {
            crash!(1, "cannot set binary and text mode at the same time");
        }
        let binary = if binary_flag { true } else if text_flag { false } else { binary_flag_default };
        let check = matches.opt_present("check");
        let tag = matches.opt_present("tag");
        let status = matches.opt_present("status");
        let quiet = matches.opt_present("quiet") || status;
        let strict = matches.opt_present("strict");
        let warn = matches.opt_present("warn") && !status;
        let files = if matches.free.is_empty() {
            vec!("-".to_owned())
        } else {
            matches.free
        };
        match hashsum(name, algo, files, binary, check, tag, status, quiet, strict, warn, bits) {
            Ok(()) => return 0,
            Err(e) => return e
        }
    }

    0
}

fn version() {
    pipe_println!("{} {}", NAME, VERSION);
}

fn usage(program: &str, binary_name: &str, opts: &getopts::Options) {
    let spec = if is_custom_binary(binary_name) {
        format!("  {} [OPTION]... [FILE]...", program)
    } else {
        format!("  {} {{--md5|--sha1|--sha224|--sha256|--sha384|--sha512|\
                        --sha3|--sha3-224|--sha3-256|--sha3-384|--sha3-512|\
                        --shake128|--shake256}} [OPTION]... [FILE]...", program)
    };

    let msg = format!("{} {}

Usage:
{}

Compute and check message digests.", NAME, VERSION, spec);

    pipe_print!("{}", opts.usage(&msg));
}

fn hashsum(algoname: &str, mut digest: Box<Digest>, files: Vec<String>, binary: bool, check: bool, tag: bool, status: bool, quiet: bool, strict: bool, warn: bool, output_bits: usize) -> Result<(), i32> {
    let mut bad_format = 0;
    let mut failed = 0;
    let binary_marker = if binary {
        "*"
    } else {
        " "
    };
    for filename in &files {
        let filename: &str = filename;
        let stdin_buf;
        let file_buf;
        let mut file = BufReader::new(
            if filename == "-" {
                stdin_buf = stdin();
                Box::new(stdin_buf) as Box<Read>
            } else {
                file_buf = safe_unwrap!(File::open(filename));
                Box::new(file_buf) as Box<Read>
            }
        );
        if check {
            // Set up Regexes for line validation and parsing
            let bytes = digest.output_bits() / 4;
            let gnu_re = safe_unwrap!(
                Regex::new(
                    &format!(
                        r"^(?P<digest>[a-fA-F0-9]{{{}}}) (?P<binary>[ \*])(?P<fileName>.*)",
                        bytes
                    )
                )
            );
            let bsd_re = safe_unwrap!(
                Regex::new(
                    &format!(
                        r"^{algorithm} \((?P<fileName>.*)\) = (?P<digest>[a-fA-F0-9]{{{digest_size}}})",
                        algorithm = algoname,
                        digest_size = bytes
                    )
                )
            );

            let buffer = file;
            for (i, line) in buffer.lines().enumerate() {
                let line = safe_unwrap!(line);
                let (ck_filename, sum, binary_check) = match gnu_re.captures(&line) {
                    Some(caps) => (caps.name("fileName").unwrap().as_str(),
                                   caps.name("digest").unwrap().as_str().to_ascii_lowercase(),
                                   caps.name("binary").unwrap().as_str() == "*"),
                    None => match bsd_re.captures(&line) {
                        Some(caps) => (caps.name("fileName").unwrap().as_str(),
                                       caps.name("digest").unwrap().as_str().to_ascii_lowercase(),
                                       true),
                        None => {
                            bad_format += 1;
                            if strict {
                                return Err(1);
                            }
                            if warn {
                                show_warning!("{}: {}: improperly formatted {} checksum line", filename, i + 1, algoname);
                            }
                            continue;
                        }
                    }
                };
                let f = safe_unwrap!(File::open(ck_filename));
                let mut ckf = BufReader::new(Box::new(f) as Box<Read>);
                let real_sum = safe_unwrap!(digest_reader(&mut digest, &mut ckf, binary_check, output_bits))
                    .to_ascii_lowercase();
                if sum == real_sum {
                    if !quiet {
                        pipe_println!("{}: OK", ck_filename);
                    }
                } else {
                    if !status {
                        pipe_println!("{}: FAILED", ck_filename);
                    }
                    failed += 1;
                }
            }
        } else {
            let sum = safe_unwrap!(digest_reader(&mut digest, &mut file, binary, output_bits));
            if tag {
                pipe_println!("{} ({}) = {}", algoname, filename, sum);
            } else {
                pipe_println!("{} {}{}", sum, binary_marker, filename);
            }
        }
    }
    if !status {
        if bad_format == 1 {
            show_warning!("{} line is improperly formatted", bad_format);
        } else if bad_format > 1 {
            show_warning!("{} lines are improperly formatted", bad_format);
        }
        if failed > 0 {
            show_warning!("{} computed checksum did NOT match", failed);
        }
    }

    Ok(())
}

fn digest_reader<'a, T: Read>(digest: &mut Box<Digest+'a>, reader: &mut BufReader<T>, binary: bool, output_bits: usize) -> io::Result<String> {
    digest.reset();

    // Digest file, do not hold too much in memory at any given moment
    let windows = cfg!(windows);
    let mut buffer = Vec::with_capacity(524288);
    let mut vec = Vec::with_capacity(524288);
    let mut looking_for_newline = false;
    loop {
        match reader.read_to_end(&mut buffer) {
            Ok(0) => { break; },
            Ok(nread) => {
                if windows && !binary {
                    // Windows text mode returns '\n' when reading '\r\n'
                    for i in 0 .. nread {
                        if looking_for_newline {
                            if buffer[i] != ('\n' as u8) {
                                vec.push('\r' as u8);
                            }
                            if buffer[i] != ('\r' as u8) {
                                vec.push(buffer[i]);
                                looking_for_newline = false;
                            }
                        } else if buffer[i] != ('\r' as u8) {
                            vec.push(buffer[i]);
                        } else {
                            looking_for_newline = true;
                        }
                    }
                    digest.input(&vec);
                    vec.clear();
                } else {
                    digest.input(&buffer[..nread]);
                }
            },
            Err(e) => return Err(e)
        }
    }
    if windows && looking_for_newline {
        vec.push('\r' as u8);
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
