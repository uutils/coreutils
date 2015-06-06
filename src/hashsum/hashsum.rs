#![crate_name = "hashsum"]

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

use crypto::digest::Digest;
use crypto::md5::Md5;
use crypto::sha1::Sha1;
use crypto::sha2::{Sha224, Sha256, Sha384, Sha512};
use regex::Regex;
use std::ascii::AsciiExt;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read, stdin, Write};
use std::path::Path;

#[path = "../common/util.rs"]
#[macro_use]
mod util;

static NAME: &'static str = "hashsum";
static VERSION: &'static str = "1.0.0";

fn is_custom_binary(program: &str) -> bool {
    match program {
        "md5sum" | "sha1sum"
            | "sha224sum" | "sha256sum"
            | "sha384sum" | "sha512sum" => true,
        _ => false
    }
}

fn detect_algo(program: &str, matches: &getopts::Matches) -> (&'static str, Box<Digest+'static>) {
    let mut alg: Option<Box<Digest>> = None;
    let mut name: &'static str = "";
    match program {
        "md5sum" => ("MD5", Box::new(Md5::new()) as Box<Digest>),
        "sha1sum" => ("SHA1", Box::new(Sha1::new()) as Box<Digest>),
        "sha224sum" => ("SHA224", Box::new(Sha224::new()) as Box<Digest>),
        "sha256sum" => ("SHA256", Box::new(Sha256::new()) as Box<Digest>),
        "sha384sum" => ("SHA384", Box::new(Sha384::new()) as Box<Digest>),
        "sha512sum" => ("SHA512", Box::new(Sha512::new()) as Box<Digest>),
        _ => {
            {
                let mut set_or_crash = |n, val| -> () {
                    if alg.is_some() { crash!(1, "You cannot combine multiple hash algorithms!") };
                    name = n;
                    alg = Some(val);
                };
                if matches.opt_present("md5") { set_or_crash("MD5", Box::new(Md5::new())) };
                if matches.opt_present("sha1") { set_or_crash("SHA1", Box::new(Sha1::new())) };
                if matches.opt_present("sha224") { set_or_crash("SHA224", Box::new(Sha224::new())) };
                if matches.opt_present("sha256") { set_or_crash("SHA256", Box::new(Sha256::new())) };
                if matches.opt_present("sha384") { set_or_crash("SHA384", Box::new(Sha384::new())) };
                if matches.opt_present("sha512") { set_or_crash("SHA512", Box::new(Sha512::new())) };
            }
            if alg.is_none() { crash!(1, "You must specify hash algorithm!") };
            (name, alg.unwrap())
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
    }

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => crash!(1, "{}", f)
    };

    if matches.opt_present("help") {
        usage(program, binary_name, &opts);
    } else if matches.opt_present("version") {
        version();
    } else {
        let (name, algo) = detect_algo(binary_name, &matches);

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
            vec!("-".to_string())
        } else {
            matches.free
        };
        match hashsum(name, algo, files, binary, check, tag, status, quiet, strict, warn) {
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
        format!("  {} {{--md5|--sha1|--sha224|--sha256|--sha384|--sha512}} [OPTION]... [FILE]...", program)
    };

    let msg = format!("{} {}

Usage:
{}

Compute and check message digests.", NAME, VERSION, spec);

    pipe_print!("{}", opts.usage(&msg));
}

fn hashsum<'a>(algoname: &str, mut digest: Box<Digest+'a>, files: Vec<String>, binary: bool, check: bool, tag: bool, status: bool, quiet: bool, strict: bool, warn: bool) -> Result<(), i32> {
    let mut bad_format = 0;
    let mut failed = 0;
    let binary_marker = if binary {
        "*"
    } else {
        " "
    };
    for filename in files.iter() {
        let filename: &str = filename;
        let mut stdin_buf;
        let mut file_buf;
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
                    Some(caps) => (caps.name("fileName").unwrap(),
                                   caps.name("digest").unwrap().to_ascii_lowercase(),
                                   caps.name("binary").unwrap() == "*"),
                    None => match bsd_re.captures(&line) {
                        Some(caps) => (caps.name("fileName").unwrap(),
                                       caps.name("digest").unwrap().to_ascii_lowercase(),
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
                let real_sum = safe_unwrap!(digest_reader(&mut digest, &mut ckf, binary_check))
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
            let sum = safe_unwrap!(digest_reader(&mut digest, &mut file, binary));
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

fn digest_reader<'a, T: Read>(digest: &mut Box<Digest+'a>, reader: &mut BufReader<T>, binary: bool) -> io::Result<String> {
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

    Ok(digest.result_str())
}
