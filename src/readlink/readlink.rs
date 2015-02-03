#![crate_name = "readlink"]
#![feature(collections, core, io, os, path, rustc_private)]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Haitao Li <lihaitao@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate getopts;

use getopts::{optflag, getopts, usage, OptGroup};
use std::old_io as io;
use std::old_io::fs;
use std::os;
use std::vec::Vec;

use CanonicalizeMode::{None, Normal, Existing, Missing};


#[path = "../common/util.rs"]
#[macro_use]
mod util;

const NAME: &'static str = "readlink";
const VERSION:  &'static str = "0.0.1";


#[derive(PartialEq)]
enum CanonicalizeMode {
    None,
    Normal,
    Existing,
    Missing,
}


fn resolve(original: &Path) -> io::IoResult<Path> {
    const MAX_LINKS_FOLLOWED: u32 = 255;
    let mut followed = 0;
    let mut result = original.clone();
    loop {
        if followed == MAX_LINKS_FOLLOWED {
            return Err(io::standard_error(io::InvalidInput));
        }

        match fs::lstat(&result) {
            Err(e) => return Err(e),
            Ok(ref stat) if stat.kind != io::FileType::Symlink => break,
            Ok(..) => {
                followed += 1;
                match fs::readlink(&result) {
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
    return Ok(result);
}


fn canonicalize(original: &Path, can_mode: &CanonicalizeMode) -> io::IoResult<Path> {
    let original = os::make_absolute(original).unwrap();
    let result = original.root_path();
    let mut result = result.expect("make_absolute has no root_path");
    let mut parts = vec![];
    for part in original.components() {
        parts.push(part);
    }

    if parts.len() > 1 {
        for part in parts.init().iter() {
            result.push(part);

            if *can_mode == None {
                continue;
            }

            match resolve(&result) {
                Err(_) => match *can_mode {
                    Missing => continue,
                    _ => return Err(io::standard_error(io::InvalidInput)),
                },
                Ok(path) => {
                    result.pop();
                    result.push(path);
                }
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
    return Ok(result);
}


pub fn uumain(args: Vec<String>) -> isize {
    let program = args[0].as_slice();
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

    let matches = match getopts(args.tail(), &opts) {
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
        let p = Path::new(f);
        if can_mode == None {
            match fs::readlink(&p) {
                Ok(path) => show(path.as_str().unwrap(), no_newline, use_zero),
                Err(_) => return 1
            }
        } else {
            match canonicalize(&p, &can_mode) {
                Ok(path) => show(path.as_str().unwrap(), no_newline, use_zero),
                Err(_) => return 1
            }
        }
    }
    return 0;
}


fn show(path: &str, no_newline: bool, use_zero: bool) {
    if use_zero {
        print!("{}\0", path);
    } else {
        if no_newline {
            io::print(path);
        } else {
            io::println(path);
        }
    }
}

fn show_usage(program: &str, opts: &[OptGroup]) {
    println!("Usage: {0} [OPTION]... [FILE]...", program);
    print!("Print value of a symbolic link or canonical file name");
    io::print(usage("", opts).as_slice());
}
