#![crate_name = "readlink"]
#![feature(file_type, rustc_private)]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Haitao Li <lihaitao@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate getopts;

use getopts::{getopts, optflag, OptGroup, usage};
use std::env;
use std::fs::{metadata, read_link};
use std::io::{Error, ErrorKind, Result, Write};
use std::path::{Component, PathBuf};

use CanonicalizeMode::{None, Normal, Existing, Missing};

#[path = "../common/util.rs"]
#[macro_use]
mod util;

const NAME: &'static str = "readlink";
const VERSION: &'static str = "0.0.1";

#[derive(PartialEq)]
enum CanonicalizeMode {
    None,
    Normal,
    Existing,
    Missing,
}

fn resolve(original: &PathBuf) -> Result<PathBuf> {
    const MAX_LINKS_FOLLOWED: u32 = 255;
    let mut followed = 0;
    let mut result = original.clone();
    loop {
        if followed == MAX_LINKS_FOLLOWED {
            return Err(Error::new(ErrorKind::InvalidInput, "maximum links followed"));
        }

        match metadata(&result) {
            Err(e) => return Err(e),
            Ok(ref m) if !m.file_type().is_symlink() => break,
            Ok(..) => {
                followed += 1;
                match read_link(&result) {
                    Ok(path) => {
                        result.pop();
                        result.push(path);
                    },
                    Err(e) => {
                        return Err(e);
                    }
                }
            }
        }
    }
    Ok(result)
}

fn canonicalize(original: &PathBuf, can_mode: &CanonicalizeMode) -> Result<PathBuf> {
    // Create an absolute path
    let original = if original.as_path().is_absolute() {
        original.clone()
    } else {
        env::current_dir().unwrap().join(original)
    };

    let mut result = PathBuf::new();
    let mut parts = vec!();

    // Split path by directory separator; add prefix (Windows-only) and root
    // directory to final path buffer; add remaining parts to temporary
    // vector for canonicalization.
    for part in original.components() {
        match part {
            Component::Prefix(_) | Component::RootDir => {
                result.push(part.as_os_str());
            },
            Component::CurDir => {},
            Component::ParentDir => {
                parts.pop();
            },
            Component::Normal(_) => {
                parts.push(part.as_os_str());
            }
        }
    }

    // Resolve the symlinks where possible
    if parts.len() > 0 {
        for part in parts[..parts.len()-1].iter() {
            result.push(part);

            if *can_mode == None {
                continue;
            }

            match resolve(&result) {
                Err(e) => match *can_mode {
                    Missing => continue,
                    _ => return Err(e)
                },
                Ok(path) => {
                    result.pop();
                    result.push(path);
                }
            }
        }

        result.push(parts.last().unwrap());

        match resolve(&result) {
            Err(e) => { if *can_mode == Existing { return Err(e); } },
            Ok(path) => {
                result.pop();
                result.push(path);
            }
        }
    }
    Ok(result)
}

pub fn uumain(args: Vec<String>) -> i32 {
    let program = &args[0];
    let opts = [
        optflag("f", "canonicalize",
                "canonicalize by following every symlink in every component of the \
                 given name recursively; all but the last component must exist"),
        optflag("e", "canonicalize-existing",
                "canonicalize by following every symlink in every component of the \
                 given name recursively, all components must exist"),
        optflag("m", "canonicalize-missing",
                "canonicalize by following every symlink in every component of the \
                 given name recursively, without requirements on components existence"),
        optflag("n", "no-newline", "do not output the trailing delimiter"),
        optflag("q", "quiet", "suppress most error messages"),
        optflag("s", "silent", "suppress most error messages"),
        optflag("v", "verbose", "report error message"),
        optflag("z", "zero", "separate output with NUL rather than newline"),
        optflag("", "help", "display this help and exit"),
        optflag("", "version", "output version information and exit"),
    ];

    let matches = match getopts(&args[1..], &opts) {
        Ok(m) => m,
        Err(f) => crash!(1, "Invalid options\n{}", f)
    };
    if matches.opt_present("help") {
        show_usage(program, &opts);
        return 0;
    }

    if matches.opt_present("version") {
        println!("{} v{}", NAME, VERSION);
        return 0;
    }

    let mut no_newline = matches.opt_present("no-newline");
    let use_zero = matches.opt_present("zero");
    let silent = matches.opt_present("silent") || matches.opt_present("quiet");
    let verbose = matches.opt_present("verbose");

    let mut can_mode = None;
    if matches.opt_present("canonicalize") {
        can_mode = Normal;
    }

    if matches.opt_present("canonicalize-existing") {
        can_mode = Existing;
    }

    if matches.opt_present("canonicalize-missing") {
        can_mode = Missing;
    }

    let files = matches.free;
    if files.len() == 0 {
        crash!(1, "missing operand\nTry {} --help for more information", program);
    }

    if no_newline && files.len() > 1 {
        if !silent {
            eprintln!("{}: ignoring --no-newline with multiple arguments", program);
            no_newline = false;
        }
    }

    for f in files.iter() {
        let p = PathBuf::from(f);
        if can_mode == None {
            match read_link(&p) {
                Ok(path) => show(&path, no_newline, use_zero),
                Err(err) => {
                    if verbose {
                        eprintln!("{}: {}: errno {}", NAME, f, err.raw_os_error().unwrap());
                    }
                    return 1
                }
            }
        } else {
            match canonicalize(&p, &can_mode) {
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
}

fn show_usage(program: &str, opts: &[OptGroup]) {
    println!("Usage: {0} [OPTION]... [FILE]...", program);
    print!("Print value of a symbolic link or canonical file name");
    print!("{}", usage("", opts));
}
