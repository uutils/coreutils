#![crate_name = "cp"]
#![feature(rustc_private, path_ext)]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Jordy Dickinson <jordy.dickinson@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE file
 * that was distributed with this source code.
 */

extern crate getopts;
#[macro_use] extern crate log;

use getopts::{getopts, optflag, usage};
use std::fs;
use std::fs::{PathExt};
use std::io::{ErrorKind, Result};
use std::path::Path;

#[derive(Clone, Eq, PartialEq)]
pub enum Mode {
    Copy,
    Help,
    Version,
}

static NAME: &'static str = "cp";
static VERSION: &'static str = "1.0.0";

pub fn uumain(args: Vec<String>) -> i32 {
    let opts = [
        optflag("h", "help", "display this help and exit"),
        optflag("", "version", "output version information and exit"),
    ];
    let matches = match getopts(&args[1..], &opts) {
        Ok(m) => m,
        Err(e) => {
            error!("error: {}", e);
            panic!()
        },
    };

    let progname = &args[0];
    let usage = usage("Copy SOURCE to DEST, or multiple SOURCE(s) to DIRECTORY.", &opts);
    let mode = if matches.opt_present("version") {
        Mode::Version
    } else if matches.opt_present("help") {
        Mode::Help
    } else {
        Mode::Copy
    };

    match mode {
        Mode::Copy    => copy(matches),
        Mode::Help    => help(&progname, &usage),
        Mode::Version => version(),
    }

    0
}

fn version() {
    println!("{} {}", NAME, VERSION);
}

fn help(progname: &String, usage: &String) {
    let msg = format!("Usage: {0} SOURCE DEST\n  \
                         or:  {0} SOURCE... DIRECTORY\n  \
                         or:  {0} -t DIRECTORY SOURCE\n\
                       \n\
                       {1}", progname, usage);
    println!("{}", msg);
}

fn copy(matches: getopts::Matches) {
    let sources: Vec<String> = if matches.free.is_empty() {
        error!("error: Missing SOURCE argument. Try --help.");
        panic!()
    } else {
        // All but the last argument:
        matches.free[..matches.free.len() - 1].iter().map(|arg| arg.clone()).collect()
    };
    let dest = if matches.free.len() < 2 {
        error!("error: Missing DEST argument. Try --help.");
        panic!()
    } else {
        // Only the last argument:
        Path::new(&matches.free[matches.free.len() - 1])
    };

    assert!(sources.len() >= 1);

    if sources.len() == 1 {
        let source = Path::new(&sources[0]);
        let same_file = paths_refer_to_same_file(source, dest).unwrap_or_else(|err| {
            match err.kind() {
                ErrorKind::NotFound => false,
                _ => {
                    error!("error: {}", err);
                    panic!()
                }
            }
        });

        if same_file {
            error!("error: \"{}\" and \"{}\" are the same file",
                source.display(),
                dest.display());
            panic!();
        }

        if let Err(err) = fs::copy(source, dest) {
            error!("error: {}", err);
            panic!();
        }
    } else {
        if !fs::metadata(dest).unwrap().is_dir() {
            error!("error: TARGET must be a directory");
            panic!();
        }

        for src in sources.iter() {
            let source = Path::new(&src);

            if !fs::metadata(source).unwrap().is_file() {
                error!("error: \"{}\" is not a file", source.display());
                continue;
            }

            let mut full_dest = dest.to_path_buf();

            full_dest.push(source.to_str().unwrap());

            println!("{}", full_dest.display());

            let io_result = fs::copy(source, full_dest);

            if let Err(err) = io_result {
                error!("error: {}", err);
                panic!()
            }
        }
    }
}

pub fn paths_refer_to_same_file(p1: &Path, p2: &Path) -> Result<bool> {
    // We have to take symlinks and relative paths into account.
    let pathbuf1 = try!(p1.canonicalize());
    let pathbuf2 = try!(p2.canonicalize());

    Ok(pathbuf1 == pathbuf2)
}
