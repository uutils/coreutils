#![crate_name = "uu_readlink"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Haitao Li <lihaitao@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate getopts;

#[macro_use]
extern crate uucore;

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use uucore::fs::{canonicalize, CanonicalizeMode};

const NAME: &'static str = "readlink";
const VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = getopts::Options::new();

    opts.optflag("f", "canonicalize",
                 "canonicalize by following every symlink in every component of the \
                  given name recursively; all but the last component must exist");
    opts.optflag("e", "canonicalize-existing",
                 "canonicalize by following every symlink in every component of the \
                  given name recursively, all components must exist");
    opts.optflag("m", "canonicalize-missing",
                 "canonicalize by following every symlink in every component of the \
                  given name recursively, without requirements on components existence");
    opts.optflag("n", "no-newline", "do not output the trailing delimiter");
    opts.optflag("q", "quiet", "suppress most error messages");
    opts.optflag("s", "silent", "suppress most error messages");
    opts.optflag("v", "verbose", "report error message");
    opts.optflag("z", "zero", "separate output with NUL rather than newline");
    opts.optflag("", "help", "display this help and exit");
    opts.optflag("", "version", "output version information and exit");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => crash!(1, "Invalid options\n{}", f)
    };
    if matches.opt_present("help") {
        show_usage(&opts);
        return 0;
    }

    if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
        return 0;
    }

    let mut no_newline = matches.opt_present("no-newline");
    let use_zero = matches.opt_present("zero");
    let silent = matches.opt_present("silent") || matches.opt_present("quiet");
    let verbose = matches.opt_present("verbose");

    let mut can_mode = CanonicalizeMode::None;
    if matches.opt_present("canonicalize") {
        can_mode = CanonicalizeMode::Normal;
    }

    if matches.opt_present("canonicalize-existing") {
        can_mode = CanonicalizeMode::Existing;
    }

    if matches.opt_present("canonicalize-missing") {
        can_mode = CanonicalizeMode::Missing;
    }

    let files = matches.free;
    if files.is_empty() {
        crash!(1, "missing operand\nTry {} --help for more information", NAME);
    }

    if no_newline && files.len() > 1 && !silent {
        eprintln!("{}: ignoring --no-newline with multiple arguments", NAME);
        no_newline = false;
    }

    for f in &files {
        let p = PathBuf::from(f);
        if can_mode == CanonicalizeMode::None {
            match fs::read_link(&p) {
                Ok(path) => show(&path, no_newline, use_zero),
                Err(err) => {
                    if verbose {
                        eprintln!("{}: {}: errno {}", NAME, f, err.raw_os_error().unwrap());
                    }
                    return 1
                }
            }
        } else {
            match canonicalize(&p, can_mode) {
                Ok(path) => show(&path, no_newline, use_zero),
                Err(err) => {
                    if verbose {
                        eprintln!("{}: {}: errno {:?}", NAME, f, err.raw_os_error().unwrap());
                    }
                    return 1
                }
            }
        }
    }

    0
}

fn show(path: &PathBuf, no_newline: bool, use_zero: bool) {
    let path = path.as_path().to_str().unwrap();
    if use_zero {
        print!("{}\0", path);
    } else if no_newline {
        print!("{}", path);
    } else {
        println!("{}", path);
    }
    pipe_flush!();
}

fn show_usage(opts: &getopts::Options) {
    println!("{} {}", NAME, VERSION);
    println!("");
    println!("Usage: {0} [OPTION]... [FILE]...", NAME);
    print!("Print value of a symbolic link or canonical file name");
    print!("{}", opts.usage(""));
}
