#![crate_id(name="rmdir", vers="1.0.0", author="Arcterus")]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Arcterus <arcterus@mail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

#![feature(macro_rules)]

extern crate getopts;
extern crate libc;

use std::os;
use std::io::{print, fs};

#[path = "../common/util.rs"]
mod util;

static NAME: &'static str = "rmdir";

#[allow(dead_code)]
fn main() { os::set_exit_status(uumain(os::args())); }

pub fn uumain(args: Vec<String>) -> int {
    let program = args.get(0).clone();

    let opts = [
        getopts::optflag("", "ignore-fail-on-non-empty", "ignore each failure that is solely because a directory is non-empty"),
        getopts::optflag("p", "parents", "remove DIRECTORY and its ancestors; e.g., 'rmdir -p a/b/c' is similar to rmdir a/b/c a/b a"),
        getopts::optflag("v", "verbose", "output a diagnostic for every directory processed"),
        getopts::optflag("h", "help", "print this help and exit"),
        getopts::optflag("V", "version", "output version information and exit")
    ];
    let matches = match getopts::getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(f) => {
            show_error!("{}", f.to_err_msg());
            return 1;
        }
    };

    if matches.opt_present("help") {
        println!("rmdir 1.0.0");
        println!("");
        println!("Usage:");
        println!("  {0:s} [OPTION]... DIRECTORY...", program);
        println!("");
        print(getopts::usage("Remove the DIRECTORY(ies), if they are empty.", opts).as_slice());
    } else if matches.opt_present("version") {
        println!("rmdir 1.0.0");
    } else if matches.free.is_empty() {
        show_error!("missing an argument");
        show_error!("for help, try '{0:s} --help'", program);
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

    return 0;
}

fn remove(dirs: Vec<String>, ignore: bool, parents: bool, verbose: bool) -> Result<(), int>{
    for dir in dirs.iter() {
        let path = Path::new(dir.as_slice());
        if path.exists() {
            if path.is_dir() {
                match remove_dir(&path, dir.as_slice(), ignore, parents, verbose) {
                    Ok(()) => ( /* pass */ ),
                    Err(e) => return Err(e)
                }
            } else {
                show_error!("failed to remove '{}' (file)", *dir);
                return Err(1);
            }
        } else {
            show_error!("no such file or directory '{}'", *dir);
            return Err(1);
        }
    }

    return Ok(());
}

fn remove_dir(path: &Path, dir: &str, ignore: bool, parents: bool, verbose: bool) -> Result<(), int> {
    let mut walk_dir = match fs::walk_dir(path) {
        Ok(m) => m,
        Err(f) => {
            show_error!("{}", f.to_str());
            return Err(1);
        }
    };
    if walk_dir.next() == None {
        match fs::rmdir(path) {
            Ok(_) => {
                if verbose {
                    println!("Removed directory '{}'", dir);
                }
                if parents {
                    let dirname = path.dirname_str().unwrap();
                    if dirname != "." {
                        match remove_dir(&Path::new(dirname), dirname, ignore, parents, verbose) {
                            Ok(()) => ( /* pass */ ),
                            Err(e) => return Err(e)
                        }
                    }
                }
            }
            Err(f) => {
                show_error!("{}", f.to_str());
                return Err(1);
            }
        }
    } else if !ignore {
        show_error!("Failed to remove directory '{}' (non-empty)", dir);
        return Err(1);
    }

    return Ok(());
}

