#[crate_id(name = "md5sum", vers = "1.0.0", author = "Arcterus")];

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Arcterus <arcterus@mail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

#[feature(macro_rules)];

extern crate crypto = "rust-crypto";
extern crate getopts;

use std::io::fs::File;
use std::io::BufferedReader;
use std::os;
use crypto::digest::Digest;

#[path = "../common/util.rs"]
mod util;

static NAME: &'static str = "md5sum";
static VERSION: &'static str = "1.0.0";

fn main() {
    let args = os::args();

    let program = args[0].clone();

    let opts = [
        getopts::optflag("b", "binary", "read in binary mode"),
        getopts::optflag("c", "check", "read MD5 sums from the FILEs and check them"),
        getopts::optflag("", "tag", "create a BSD-style checksum"),
        getopts::optflag("t", "text", "read in text mode (default)"),
        getopts::optflag("q", "quiet", "don't print OK for each successfully verified file"),
        getopts::optflag("s", "status", "don't output anything, status code shows success"),
        getopts::optflag("", "strict", "exit non-zero for improperly formatted checksum lines"),
        getopts::optflag("w", "warn", "warn about improperly formatted checksum lines"),
        getopts::optflag("h", "help", "display this help and exit"),
        getopts::optflag("V", "version", "output version information and exit")
    ];

    let matches = match getopts::getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(f) => crash!(1, "{}", f.to_err_msg())
    };

    if matches.opt_present("help") {
        println!("{} v{}", NAME, VERSION);
        println!("");
        println!("Usage:");
        println!("  {} [OPTION]... [FILE]...", program);
        println!("");
        print!("{}", getopts::usage("Compute and check MD5 message digests.", opts));
    } else if matches.opt_present("version") {
        println!("{} v{}", NAME, VERSION);
    } else {
        let binary = matches.opt_present("binary");
        let check = matches.opt_present("check");
        let tag = matches.opt_present("tag");
        let status = matches.opt_present("status");
        let quiet = matches.opt_present("quiet") || status;
        let strict = matches.opt_present("strict");
        let warn = matches.opt_present("warn") && !status;
        md5sum(matches.free, binary, check, tag, status, quiet, strict, warn);
    }
}

fn md5sum(files: Vec<~str>, binary: bool, check: bool, tag: bool, status: bool, quiet: bool, strict: bool, warn: bool) {
    let mut md5 = crypto::md5::Md5::new();
    let bytes = md5.output_bits() / 4;
    let mut bad_format = 0;
    let mut failed = 0;
    for filename in files.iter() {
        let filename: &str = *filename;
        let mut file = safe_unwrap!(File::open(&Path::new(filename)));
        if check {
            let mut buffer = BufferedReader::new(file);
            for (i, line) in buffer.lines().enumerate() {
                let line = safe_unwrap!(line);
                let (ck_filename, sum) = match from_gnu(line, bytes) {
                    Some(m) => m,
                    None => match from_bsd(line, bytes) {
                        Some(m) => m,
                        None => {
                            bad_format += 1;
                            if strict {
                                os::set_exit_status(1);
                            }
                            if warn {
                                show_warning!("{}: {}: improperly formatted MD5 checksum line", filename, i + 1);
                            }
                            continue;
                        }
                    }
                };
                let real_sum = calc_sum(&mut md5, &mut safe_unwrap!(File::open(&Path::new(ck_filename))), binary);
                if sum == real_sum {
                    if !quiet {
                        println!("{}: OK", ck_filename);
                    }
                } else {
                    if !status {
                        println!("{}: FAILED", ck_filename);
                    }
                    failed += 1;
                    os::set_exit_status(1);
                }
            }
        } else {
            let sum = calc_sum(&mut md5, &mut file, binary);
            if tag {
                println!("MD5 ({}) = {}", filename, sum);
            } else {
                println!("{}  {}", sum, filename);
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
}

fn calc_sum(md5: &mut crypto::md5::Md5, file: &mut File, binary: bool) -> ~str {
    let data =
        if binary {
            safe_unwrap!(file.read_to_end())
        } else {
            (safe_unwrap!(file.read_to_str())).into_bytes()
        };
    md5.reset();
    md5.input(data);
    md5.result_str()
}

fn from_gnu<'a>(line: &'a str, bytes: uint) -> Option<(&'a str, &'a str)> {
    let sum = line.slice_to(bytes);
    if sum.len() < bytes || line.slice(bytes, bytes + 2) != "  " {
        None
    } else {
        Some((line.slice(bytes + 2, line.len() - 1), sum))
    }
}

fn from_bsd<'a>(line: &'a str, bytes: uint) -> Option<(&'a str, &'a str)> {
    if line.slice(0, 5) == "MD5 (" {
        let rparen = match line.find(')') {
            Some(m) => m,
            None => return None
        };
        if rparen > 5 && line.slice(rparen + 1, rparen + 4) == " = " && line.len() - 1 == rparen + 4 + bytes {
            return Some((line.slice(5, rparen), line.slice(rparen + 4, line.len() - 1)));
        }
    }
    None
}
