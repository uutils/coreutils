#![crate_name = "rmdir"]
#![feature(collections, core, io, libc, path, rustc_private)]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Arcterus <arcterus@mail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate getopts;
extern crate libc;

use std::old_io::{print, fs};
use std::old_io::fs::PathExtensions;

#[path = "../common/util.rs"]
#[macro_use]
mod util;

static NAME: &'static str = "rmdir";

pub fn uumain(args: Vec<String>) -> isize {
    let program = args[0].clone();

    let opts = [
        getopts::optflag("", "ignore-fail-on-non-empty", "ignore each failure that is solely because a directory is non-empty"),
        getopts::optflag("p", "parents", "remove DIRECTORY and its ancestors; e.g., 'rmdir -p a/b/c' is similar to rmdir a/b/c a/b a"),
        getopts::optflag("v", "verbose", "output a diagnostic for every directory processed"),
        getopts::optflag("h", "help", "print this help and exit"),
        getopts::optflag("V", "version", "output version information and exit")
    ];
    let matches = match getopts::getopts(args.tail(), &opts) {
        Ok(m) => m,
        Err(f) => {
            show_error!("{}", f);
            return 1;
        }
    };

    if matches.opt_present("help") {
        println!("rmdir 1.0.0");
        println!("");
        println!("Usage:");
        println!("  {0} [OPTION]... DIRECTORY...", program);
        println!("");
        print(getopts::usage("Remove the DIRECTORY(ies), if they are empty.", &opts).as_slice());
    } else if matches.opt_present("version") {
        println!("rmdir 1.0.0");
    } else if matches.free.is_empty() {
        show_error!("missing an argument");
        show_error!("for help, try '{0} --help'", program);
        return 1;
    } else {
        let ignore = matches.opt_present("ignore-fail-on-non-empty");
        let parents = matches.opt_present("parents");
        let verbose = matches.opt_present("verbose");
        match remove(matches.free, ignore, parents, verbose) {
            Ok(()) => ( /* pass */ ),
            Err(e) => return e
        }
    }

    0
}

fn remove(dirs: Vec<String>, ignore: bool, parents: bool, verbose: bool) -> Result<(), isize>{
    let mut r = Ok(());

    for dir in dirs.iter() {
        let path = Path::new(dir.as_slice());
        if path.exists() {
            if path.is_dir() {
                r = remove_dir(&path, dir.as_slice(), ignore, parents, verbose).and(r);
            } else {
                show_error!("failed to remove '{}' (file)", *dir);
                r = Err(1);
            }
        } else {
            show_error!("no such file or directory '{}'", *dir);
            r = Err(1);
        }
    }

    r
}

fn remove_dir(path: &Path, dir: &str, ignore: bool, parents: bool, verbose: bool) -> Result<(), isize> {
    let mut walk_dir = match fs::walk_dir(path) {
        Ok(m) => m,
        Err(f) => {
            show_error!("{}", f.to_string());
            return Err(1);
        }
    };

    let mut r = Ok(());

    if walk_dir.next() == None {
        match fs::rmdir(path) {
            Ok(_) => {
                if verbose {
                    println!("Removed directory '{}'", dir);
                }
                if parents {
                    let dirname = path.dirname_str().unwrap();
                    if dirname != "." {
                        r = remove_dir(&Path::new(dirname), dirname, ignore, parents, verbose).and(r);
                    }
                }
            }
            Err(f) => {
                show_error!("{}", f.to_string());
                r = Err(1);
            }
        }
    } else if !ignore {
        show_error!("Failed to remove directory '{}' (non-empty)", dir);
        r = Err(1);
    }

    r
}
