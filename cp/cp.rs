#![crate_id(name="cp", vers="1.0.0", author="Jordy Dickinson")]
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
#[phase(syntax, link)] extern crate log;

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

#[allow(dead_code)]
fn main() { uumain(os::args()); }

pub fn uumain(args: Vec<String>) {
    let opts = [
        optflag("h", "help", "display this help and exit"),
        optflag("", "version", "output version information and exit"),
    ];
    let matches = match getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(e) => {
            error!("error: {:s}", e.to_err_msg());
            fail!()
        },
    };

    let progname = args.get(0);
    let usage = usage("Copy SOURCE to DEST, or multiple SOURCE(s) to DIRECTORY.", opts);
    let mode = if matches.opt_present("version") {
        Version
    } else if matches.opt_present("help") {
        Help
    } else {
        Copy
    };

    match mode {
        Copy    => copy(matches),
        Help    => help(progname.as_slice(), usage.as_slice()),
        Version => version(),
    }
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
        fail!()
    } else {
        // All but the last argument:
        matches.free.slice(0, matches.free.len() - 2).iter()
            .map(|arg| Path::new(arg.clone())).collect()
    };
    let dest = if matches.free.len() < 2 {
        error!("error: Missing DEST argument. Try --help.");
        fail!()
    } else {
        // Only the last argument:
        Path::new(matches.free.get(matches.free.len() - 1).as_slice())
    };

    assert!(sources.len() >= 1);

    if sources.len() == 1 {
        let source = sources.get(0);
        let same_file = match paths_refer_to_same_file(source, &dest) {
            Ok(b)  => b,
            Err(e) => if e.kind == io::FileNotFound {
                false
            } else {
                error!("error: {:s}", e.to_str());
                fail!()
            }
        };

        if same_file {
            error!("error: \"{:s}\" and \"{:s}\" are the same file",
                source.display().to_str(),
                dest.display().to_str());
            fail!();
        }

        let io_result = fs::copy(source, &dest);

        if io_result.is_err() {
            let err = io_result.unwrap_err();
            error!("error: {:s}", err.to_str());
            fail!();
        }
    } else {
        if fs::stat(&dest).unwrap().kind != io::TypeDirectory {
            error!("error: TARGET must be a directory");
            fail!();
        }

        for source in sources.iter() {
            if fs::stat(source).unwrap().kind != io::TypeFile {
                error!("error: \"{:s}\" is not a file", source.display().to_str());
                continue;
            }

            let mut full_dest = dest.clone();

            full_dest.push(source.filename_str().unwrap());

            println!("{:s}", full_dest.display().to_str());

            let io_result = fs::copy(source, &full_dest);

            if io_result.is_err() {
                let err = io_result.unwrap_err();
                error!("error: {:s}", err.to_str());
                fail!()
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
    raw_p1 = os::make_absolute(&raw_p1);

    if p2_lstat.kind == io::TypeSymlink {
        raw_p2 = fs::readlink(&raw_p2).unwrap();
    }
    raw_p2 = os::make_absolute(&raw_p2);

    Ok(raw_p1 == raw_p2)
}
