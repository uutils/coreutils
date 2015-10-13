#![crate_name = "cp"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Jordy Dickinson <jordy.dickinson@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE file
 * that was distributed with this source code.
 */

extern crate getopts;

use getopts::Options;
use std::fs;
use std::io::{ErrorKind, Result, Write};
use std::path::Path;

#[path = "../common/util.rs"]
#[macro_use]
mod util;

#[path = "../common/filesystem.rs"]
mod filesystem;

use filesystem::{canonicalize, CanonicalizeMode, UUPathExt};

#[derive(Clone, Eq, PartialEq)]
pub enum Mode {
    Copy,
    Help,
    Version,
}

static NAME: &'static str = "cp";
static VERSION: &'static str = "1.0.0";

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = Options::new();

    opts.optflag("h", "help", "display this help and exit");
    opts.optflag("", "version", "output version information and exit");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(e) => {
            show_error!("{}", e);
            panic!()
        },
    };

    let usage = opts.usage("Copy SOURCE to DEST, or multiple SOURCE(s) to DIRECTORY.");
    let mode = if matches.opt_present("version") {
        Mode::Version
    } else if matches.opt_present("help") {
        Mode::Help
    } else {
        Mode::Copy
    };

    match mode {
        Mode::Copy    => copy(matches),
        Mode::Help    => help(&usage),
        Mode::Version => version(),
    }

    0
}

fn version() {
    println!("{} {}", NAME, VERSION);
}

fn help(usage: &String) {
    let msg = format!("{0} {1}\n\n\
                       Usage: {0} SOURCE DEST\n  \
                         or:  {0} SOURCE... DIRECTORY\n  \
                         or:  {0} -t DIRECTORY SOURCE\n\
                       \n\
                       {2}", NAME, VERSION, usage);
    println!("{}", msg);
}

fn copy(matches: getopts::Matches) {
    let sources: Vec<String> = if matches.free.is_empty() {
        show_error!("Missing SOURCE argument. Try --help.");
        panic!()
    } else {
        // All but the last argument:
        matches.free[..matches.free.len() - 1].iter().map(|arg| arg.clone()).collect()
    };
    let dest = if matches.free.len() < 2 {
        show_error!("Missing DEST argument. Try --help.");
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
                    show_error!("{}", err);
                    panic!()
                }
            }
        });

        if same_file {
            show_error!("\"{}\" and \"{}\" are the same file",
                source.display(),
                dest.display());
            panic!();
        }

        if let Err(err) = fs::copy(source, dest) {
            show_error!("{}", err);
            panic!();
        }
    } else {
        if !dest.uu_is_dir() {
            show_error!("TARGET must be a directory");
            panic!();
        }

        for src in sources.iter() {
            let source = Path::new(&src);

            if !source.uu_is_file() {
                show_error!("\"{}\" is not a file", source.display());
                continue;
            }

            let mut full_dest = dest.to_path_buf();

            full_dest.push(source.to_str().unwrap());

            println!("{}", full_dest.display());

            let io_result = fs::copy(source, full_dest);

            if let Err(err) = io_result {
                show_error!("{}", err);
                panic!()
            }
        }
    }
}

pub fn paths_refer_to_same_file(p1: &Path, p2: &Path) -> Result<bool> {
    // We have to take symlinks and relative paths into account.
    let pathbuf1 = try!(canonicalize(p1, CanonicalizeMode::Normal));
    let pathbuf2 = try!(canonicalize(p2, CanonicalizeMode::Normal));

    Ok(pathbuf1 == pathbuf2)
}
