#![crate_name = "uu_cp"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Jordy Dickinson <jordy.dickinson@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE file
 * that was distributed with this source code.
 */

extern crate getopts;

#[macro_use]
extern crate uucore;

use getopts::Options;
use std::fs;
use std::io::{ErrorKind, Result, Write};
use std::path::Path;
use uucore::fs::{canonicalize, CanonicalizeMode};

#[derive(Clone, Eq, PartialEq)]
pub enum Mode {
    Copy,
    Help,
    Version,
}

static NAME: &'static str = "cp";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = Options::new();

    opts.optflag("h", "help", "display this help and exit");
    opts.optflag("", "version", "output version information and exit");
    opts.optopt("t", "target-directory", "copy all SOURCE arguments into DIRECTORY", "DEST");
    opts.optflag("T", "no-target-directory", "Treat DEST as a regular file and not a directory");
    opts.optflag("v", "verbose", "explicitly state what is being done");

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

fn help(usage: &str) {
    let msg = format!("{0} {1}\n\n\
                       Usage: {0} SOURCE DEST\n  \
                         or:  {0} SOURCE... DIRECTORY\n  \
                         or:  {0} -t DIRECTORY SOURCE...\n\
                       \n\
                       {2}", NAME, VERSION, usage);
    println!("{}", msg);
}

fn copy(matches: getopts::Matches) {
    let verbose = matches.opt_present("verbose");
    let sources: Vec<String> = if matches.free.is_empty() {
        show_error!("Missing SOURCE or DEST argument. Try --help.");
        panic!()
    } else if !matches.opt_present("target-directory") {
        matches.free[..matches.free.len() - 1].iter().cloned().collect()
    } else {
        matches.free.iter().cloned().collect()
    };
    let dest_str = if matches.opt_present("target-directory") {
        matches.opt_str("target-directory").expect("Option -t/--target-directory requires an argument")
    } else {
        matches.free[matches.free.len() - 1].clone()
    };
    let dest = if matches.free.len() < 2 && !matches.opt_present("target-directory") {
        show_error!("Missing DEST argument. Try --help.");
        panic!()
    } else {
        //the argument to the -t/--target-directory= options
        let path = Path::new(&dest_str);
        if !path.is_dir() && matches.opt_present("target-directory") {
            show_error!("Target {} is not a directory", matches.opt_str("target-directory").unwrap());
            panic!()
        } else {
            path
        }

    };

    assert!(sources.len() >= 1);
    if matches.opt_present("no-target-directory") && dest.is_dir() {
        show_error!("Can't overwrite directory {} with non-directory", dest.display());
        panic!()
    }

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
        let mut full_dest = dest.to_path_buf();
        if dest.is_dir() {
            full_dest.push(source.file_name().unwrap()); //the destination path is the destination
        } // directory + the file name we're copying
        if verbose {
            println!("{} -> {}", source.display(), full_dest.display());
        }
        if let Err(err) = fs::copy(source, full_dest) {
            show_error!("{}", err);
            panic!();
        }
    } else {
        if !dest.is_dir() {
            show_error!("TARGET must be a directory");
            panic!();
        }
        for src in &sources {
            let source = Path::new(&src);

            if !source.is_file() {
                show_error!("\"{}\" is not a file", source.display());
                continue;
            }

            let mut full_dest = dest.to_path_buf();

            full_dest.push(source.file_name().unwrap());

            if verbose {
                println!("{} -> {}", source.display(), full_dest.display());
            }

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
