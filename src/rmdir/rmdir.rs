#![crate_name = "uu_rmdir"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Arcterus <arcterus@mail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate getopts;

#[macro_use]
extern crate uucore;

use std::fs;
use std::io::Write;
use std::path::Path;

static NAME: &'static str = "rmdir";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = getopts::Options::new();

    opts.optflag("", "ignore-fail-on-non-empty", "ignore each failure that is solely because a directory is non-empty");
    opts.optflag("p", "parents", "remove DIRECTORY and its ancestors; e.g., 'rmdir -p a/b/c' is similar to rmdir a/b/c a/b a");
    opts.optflag("v", "verbose", "output a diagnostic for every directory processed");
    opts.optflag("h", "help", "print this help and exit");
    opts.optflag("V", "version", "output version information and exit");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            show_error!("{}", f);
            return 1;
        }
    };

    if matches.opt_present("help") {
        let msg = format!("{0} {1}

Usage:
  {0} [OPTION]... DIRECTORY...

Remove the DIRECTORY(ies), if they are empty.", NAME, VERSION);
        print!("{}", opts.usage(&msg));
    } else if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
    } else if matches.free.is_empty() {
        show_error!("missing an argument");
        show_error!("for help, try '{0} --help'", NAME);
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

fn remove(dirs: Vec<String>, ignore: bool, parents: bool, verbose: bool) -> Result<(), i32> {
    let mut r = Ok(());

    for dir in &dirs {
        let path = Path::new(&dir[..]);
        r = remove_dir(&path, ignore, verbose).and(r);
        if parents {
            let mut p = path;
            while let Some(new_p) = p.parent() {
                p = new_p;
                match p.as_os_str().to_str() {
                    None => break,
                    Some(s) => match s {
                        "" | "." | "/" => break,
                        _ => (),
                    },
                };
                r = remove_dir(p, ignore, verbose).and(r);
            }
        }
    }

    r
}

fn remove_dir(path: &Path, ignore: bool, verbose: bool) -> Result<(), i32> {
    let mut read_dir = match fs::read_dir(path) {
        Ok(m) => m,
        Err(e) => {
            show_error!("reading directory '{}': {}", path.display(), e);
            return Err(1);
        }
    };

    let mut r = Ok(());

    if read_dir.next().is_none() {
        match fs::remove_dir(path) {
            Err(e) => {
                show_error!("removing directory '{}': {}", path.display(), e);
                r = Err(1);
            },
            Ok(_) if verbose => println!("Removed directory '{}'", path.display()),
            _ => (),
        }
    } else if !ignore {
        show_error!("failed to remove '{}': Directory not empty", path.display());
        r = Err(1);
    }

    r
}
