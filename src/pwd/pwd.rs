#![crate_name = "uu_pwd"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Derek Chiang <derekchiang93@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate getopts;

#[macro_use]
extern crate uucore;

use std::env;
use std::path::{Path, PathBuf};
use std::io;

static NAME: &str = "pwd";
static VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn absolute_path(path: &Path) -> io::Result<PathBuf> {
    let path_buf = path.canonicalize()?;

    #[cfg(windows)]
    let path_buf = Path::new(
        path_buf
            .as_path()
            .to_string_lossy()
            .trim_start_matches(r"\\?\"),
    ).to_path_buf();

    Ok(path_buf)
}

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = getopts::Options::new();

    opts.optflag("", "help", "display this help and exit");
    opts.optflag("", "version", "output version information and exit");
    opts.optflag(
        "L",
        "logical",
        "use PWD from environment, even if it contains symlinks",
    );
    opts.optflag("P", "physical", "avoid all symlinks");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => crash!(1, "Invalid options\n{}", f),
    };

    if matches.opt_present("help") {
        let msg = format!(
            "{0} {1}

Usage:
  {0} [OPTION]...

Print the full filename of the current working directory.",
            NAME, VERSION
        );
        print!("{}", opts.usage(&msg));
    } else if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
    } else {
        match env::current_dir() {
            Ok(logical_path) => {
                if matches.opt_present("logical") {
                    println!("{}", logical_path.display());
                } else {
                    match absolute_path(&logical_path) {
                        Ok(physical_path) => println!("{}", physical_path.display()),
                        Err(e) => crash!(1, "failed to get absolute path {}", e),
                    };
                }
            }
            Err(e) => crash!(1, "failed to get current directory {}", e),
        };
    }

    0
}
