#![crate_name = "cp"]
#![feature(macro_rules)]
#![feature(phase)]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Jordy Dickinson <jordy.dickinson@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE file
 * that was distributed with this source code.
 */

extern crate getopts;
#[phase(plugin, link)] extern crate log;

use std::os;
use std::io;
use std::io::fs;

use getopts::{
    getopts,
    optflag,
    usage,
};

#[deriving(Eq, PartialEq)]
pub enum Mode {
    Copy,
    Help,
    Version,
}

pub fn uumain(args: Vec<String>) -> int {
    let opts = [
        optflag("h", "help", "display this help and exit"),
        optflag("", "version", "output version information and exit"),
    ];
    let matches = match getopts(args.tail(), &opts) {
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
        Mode::Help    => help(progname.as_slice(), usage.as_slice()),
        Mode::Version => version(),
    }

    0
}

fn version() {
    println!("cp 1.0.0");
}

fn help(progname: &str, usage: &str) {
    let msg = format!("Usage: {0} SOURCE DEST\n  \
                         or:  {0} SOURCE... DIRECTORY\n  \
                         or:  {0} -t DIRECTORY SOURCE\n\
                       \n\
                       {1}", progname, usage);
    println!("{}", msg);
}

fn copy(matches: getopts::Matches) {
    let sources : Vec<Path> = if matches.free.len() < 1 {
        error!("error: Missing SOURCE argument. Try --help.");
        panic!()
    } else {
        // All but the last argument:
        matches.free.slice(0, matches.free.len() - 1).iter()
            .map(|arg| Path::new(arg.clone())).collect()
    };
    let dest = if matches.free.len() < 2 {
        error!("error: Missing DEST argument. Try --help.");
        panic!()
    } else {
        // Only the last argument:
        Path::new(matches.free[matches.free.len() - 1].as_slice())
    };

    assert!(sources.len() >= 1);

    if sources.len() == 1 {
        let source = &sources[0];
        let same_file = match paths_refer_to_same_file(source, &dest) {
            Ok(b)  => b,
            Err(e) => if e.kind == io::FileNotFound {
                false
            } else {
                error!("error: {}", e.to_string());
                panic!()
            }
        };

        if same_file {
            error!("error: \"{}\" and \"{}\" are the same file",
                source.display().to_string(),
                dest.display().to_string());
            panic!();
        }

        let io_result = fs::copy(source, &dest);

        if io_result.is_err() {
            let err = io_result.unwrap_err();
            error!("error: {}", err.to_string());
            panic!();
        }
    } else {
        if fs::stat(&dest).unwrap().kind != io::TypeDirectory {
            error!("error: TARGET must be a directory");
            panic!();
        }

        for source in sources.iter() {
            if fs::stat(source).unwrap().kind != io::TypeFile {
                error!("error: \"{}\" is not a file", source.display().to_string());
                continue;
            }

            let mut full_dest = dest.clone();

            full_dest.push(source.filename_str().unwrap());

            println!("{}", full_dest.display().to_string());

            let io_result = fs::copy(source, &full_dest);

            if io_result.is_err() {
                let err = io_result.unwrap_err();
                error!("error: {}", err.to_string());
                panic!()
            }
        }
    }
}

pub fn paths_refer_to_same_file(p1: &Path, p2: &Path) -> io::IoResult<bool> {
    let mut raw_p1 = p1.clone();
    let mut raw_p2 = p2.clone();

    let p1_lstat = match fs::lstat(&raw_p1) {
        Ok(stat) => stat,
        Err(e)   => return Err(e),
    };

    let p2_lstat = match fs::lstat(&raw_p2) {
        Ok(stat) => stat,
        Err(e)   => return Err(e),
    };

    // We have to take symlinks and relative paths into account.
    if p1_lstat.kind == io::TypeSymlink {
        raw_p1 = fs::readlink(&raw_p1).unwrap();
    }
    raw_p1 = os::make_absolute(&raw_p1).unwrap();

    if p2_lstat.kind == io::TypeSymlink {
        raw_p2 = fs::readlink(&raw_p2).unwrap();
    }
    raw_p2 = os::make_absolute(&raw_p2).unwrap();

    Ok(raw_p1 == raw_p2)
}
