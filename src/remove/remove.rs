#![crate_name = "uu_remove"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Smigle00 <smigle00@mail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate getopts;
extern crate libc;

#[macro_use]
extern crate uucore;

use std::fs;
use std::io::Write;
use std::path::Path;

static NAME: &'static str = "remove";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = getopts::Options::new();

    opts.optflag("", "help", "print this help and exit");
    opts.optflag("", "version", "output version information and exit");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            show_error!("{}", f);
            return 1;
        }
    };

    if matches.opt_present("help") {
        println!("{} {}\n", NAME, VERSION);
        println!("Usage:");
        println!(" {} [OPTION]... FILE ...\n", NAME);
        print!("{}", opts.usage("Remove file or empty directory."));
    } else if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
    } else if matches.free.is_empty() {
        show_error!("missing an argument");
        show_error!("Try '{} --help' for more information.", NAME);
        return 1;
    } else {
        match remove(matches.free) {
            Ok(()) => ( /* pass */ ),
            Err(e) => return e
        }
    }

    0
}

fn remove(path_list: Vec<String>) -> Result<(), i32> {
    let mut r = Ok(());

    for path in &path_list {
        let file = Path::new(path);

        if !file.exists() {
            show_error!("failed to remove '{}': No such file or directory", path);
            r = Err(1);
            continue;
        }

        if file.is_dir() {
            r = remove_dir(&file);
        } else {
            r = remove_file(&file);
        }
    }

    r
}

fn remove_dir(path: &Path) -> Result<(), i32> {
    let mut r = Ok(());

    match fs::remove_dir(path) {
        Err(e) => {
            show_error!("failed to remove '{}' directory: {}", path.display(), e);
            r = Err(1);
        },
        Ok(()) => ( /* pass */ ),
    }

    r
}

fn remove_file(path: &Path) -> Result<(), i32> {
    let mut r = Ok(());

    match fs::remove_file(path) {
        Err(e) => {
            show_error!("failed to remove '{}' file: {}", path.display(), e);
            r = Err(1);
        },
        Ok(()) => ( /* pass */ ),
    }

    r
}
