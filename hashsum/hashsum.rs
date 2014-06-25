#![crate_id = "hashsum#1.0.0"]

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

#![feature(macro_rules)]

extern crate regex;

extern crate crypto = "rust-crypto";
extern crate getopts;

use std::io::fs::File;
use std::io::stdio::stdin_raw;
use std::io::BufferedReader;
use std::os;
use regex::Regex;
use crypto::digest::Digest;
use crypto::md5::Md5;
use crypto::sha1::Sha1;
use crypto::sha2::{Sha224, Sha256, Sha384, Sha512};

#[path = "../common/util.rs"]
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

fn get_algo_opts(program: &str) -> Vec<getopts::OptGroup> {
    if is_custom_binary(program) {
        Vec::new()
    } else {
        vec!(
            getopts::optflag("", "md5", "work with MD5"),
            getopts::optflag("", "sha1", "work with SHA1"),
            getopts::optflag("", "sha224", "work with SHA224"),
            getopts::optflag("", "sha256", "work with SHA256"),
            getopts::optflag("", "sha384", "work with SHA384"),
            getopts::optflag("", "sha512", "work with SHA512")
        )
    }
}

fn detect_algo(program: &str, matches: &getopts::Matches) -> (&str, Box<Digest>) {
    let mut alg: Option<Box<Digest>> = None;
    let mut name: &'static str = "";
    match program {
        "md5sum" => ("MD5", box Md5::new() as Box<Digest>),
        "sha1sum" => ("SHA1", box Sha1::new() as Box<Digest>),
        "sha224sum" => ("SHA224", box Sha224::new() as Box<Digest>),
        "sha256sum" => ("SHA256", box Sha256::new() as Box<Digest>),
        "sha384sum" => ("SHA384", box Sha384::new() as Box<Digest>),
        "sha512sum" => ("SHA512", box Sha512::new() as Box<Digest>),
        _ => {
            {
                let set_or_crash = |n: &'static str, val: Box<Digest>| -> () {
                    if alg.is_some() { crash!(1, "You cannot combine multiple hash algorithms!") };
                    name = n;
                    alg = Some(val);
                };
                if matches.opt_present("md5") { set_or_crash("MD5", box Md5::new()) };
                if matches.opt_present("sha1") { set_or_crash("SHA1", box Sha1::new()) };
                if matches.opt_present("sha224") { set_or_crash("SHA224", box Sha224::new()) };
                if matches.opt_present("sha256") { set_or_crash("SHA256", box Sha256::new()) };
                if matches.opt_present("sha384") { set_or_crash("SHA384", box Sha384::new()) };
                if matches.opt_present("sha512") { set_or_crash("SHA512", box Sha512::new()) };
            }
            if alg.is_none() { crash!(1, "You must specify hash algorithm!") };
            (name, alg.unwrap())
        }
    }
}

#[allow(dead_code)]
fn main() { os::set_exit_status(uumain(os::args())); }

pub fn uumain(args: Vec<String>) -> int {
    let program = args.get(0).clone();
    let binary = Path::new(program.as_slice());
    let binary_name = binary.filename_str().unwrap();

    let mut opts: Vec<getopts::OptGroup> = vec!(
        getopts::optflag("b", "binary", "read in binary mode"),
        getopts::optflag("c", "check", "read hashsums from the FILEs and check them"),
        getopts::optflag("", "tag", "create a BSD-style checksum"),
        getopts::optflag("t", "text", "read in text mode (default)"),
        getopts::optflag("q", "quiet", "don't print OK for each successfully verified file"),
        getopts::optflag("s", "status", "don't output anything, status code shows success"),
        getopts::optflag("", "strict", "exit non-zero for improperly formatted checksum lines"),
        getopts::optflag("w", "warn", "warn about improperly formatted checksum lines"),
        getopts::optflag("h", "help", "display this help and exit"),
        getopts::optflag("V", "version", "output version information and exit"),
    );

    opts.push_all_move(get_algo_opts(binary_name.as_slice()));

    let matches = match getopts::getopts(args.tail(), opts.as_slice()) {
        Ok(m) => m,
        Err(f) => crash!(1, "{}", f)
    };

    if matches.opt_present("help") {
        usage(program.as_slice(), binary_name.as_slice(), opts.as_slice());
    } else if matches.opt_present("version") {
        version();
    } else {
        let (name, algo) = detect_algo(binary_name.as_slice(), &matches);

        let binary = matches.opt_present("binary");
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
    println!("{} v{}", NAME, VERSION);
}

fn usage(program: &str, binary_name: &str, opts: &[getopts::OptGroup]) {
    version();
    println!("");
    println!("Usage:");
    if is_custom_binary(binary_name) {
        println!("  {} [OPTION]... [FILE]...", program);
    } else {
        println!("  {} {{--md5|--sha1|--sha224|--sha256|--sha384|--sha512}} [OPTION]... [FILE]...", program);
    }
    println!("");
    print!("{}", getopts::usage("Compute and check message digests.", opts));
}

fn hashsum(algoname: &str, mut digest: Box<Digest>, files: Vec<String>, binary: bool, check: bool, tag: bool, status: bool, quiet: bool, strict: bool, warn: bool) -> Result<(), int> {
    let mut bad_format = 0;
    let mut failed = 0;
    let binary_marker = if binary {
        "*"
    } else {
        " "
    };
    for filename in files.iter() {
        let filename: &str = filename.as_slice();
        let mut file = BufferedReader::new(
            if filename == "-" {
                box stdin_raw() as Box<Reader>
            } else {
                box safe_unwrap!(File::open(&Path::new(filename))) as Box<Reader>
            }
        );
        if check {

            // Set up Regexs for line validation and parsing
            let bytes = digest.output_bits() / 4;
            let gnu_re = safe_unwrap!(Regex::new(format!(
                        r"^(?P<digest>[a-fA-F0-9]{{{}}}) (?P<binary>[ \*])(?P<fileName>.*)",
                        bytes
                        ).as_slice()));
            let bsd_re = safe_unwrap!(Regex::new(format!(
                        r"^{algorithm} \((?P<fileName>.*)\) = (?P<digest>[a-fA-F0-9]{{{digest_size}}})",
                        algorithm = algoname,
                        digest_size = bytes
                        ).as_slice()));

            let mut buffer = file;
            for (i, line) in buffer.lines().enumerate() {
                let line = safe_unwrap!(line);
                let (ck_filename, sum, binary_check) = match gnu_re.captures(line.as_slice()) {
                    Some(caps) => (caps.name("fileName"), caps.name("digest").to_ascii().to_lower(), caps.name("binary") == "*"),
                    None => match bsd_re.captures(line.as_slice()) {
                        Some(caps) => (caps.name("fileName"), caps.name("digest").to_ascii().to_lower(), true),
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
                let real_sum = calc_sum(&mut digest, &mut safe_unwrap!(File::open(&Path::new(ck_filename))), binary_check)
                    .as_slice().to_ascii().to_lower();
                if sum.as_slice() == real_sum.as_slice() {
                    if !quiet {
                        println!("{}: OK", ck_filename);
                    }
                } else {
                    if !status {
                        println!("{}: FAILED", ck_filename);
                    }
                    failed += 1;
                }
            }
        } else {
            let sum = calc_sum(&mut digest, &mut file, binary);
            if tag {
                println!("{} ({}) = {}", algoname, filename, sum);
            } else {
                println!("{} {}{}", sum, binary_marker, filename);
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

fn calc_sum(digest: &mut Box<Digest>, file: &mut Reader, binary: bool) -> String {
    let data =
        if binary {
            (safe_unwrap!(file.read_to_end()))
        } else {
            (safe_unwrap!(file.read_to_str())).into_bytes()
        };
    digest.reset();
    digest.input(data.as_slice());
    digest.result_str()
}

