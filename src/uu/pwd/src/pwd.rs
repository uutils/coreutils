//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Derek Chiang <derekchiang93@gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

extern crate clap;

#[macro_use]
extern crate uucore;

use clap::{App, Arg};
use std::env;
use std::io;
use std::path::{Path, PathBuf};

static ABOUT: &str = "Display the full filename of the current working directory.";
static VERSION: &str = env!("CARGO_PKG_VERSION");
static OPT_LOGICAL: &str = "logical";
static OPT_PHYSICAL: &str = "physical";

pub fn absolute_path(path: &Path) -> io::Result<PathBuf> {
    let path_buf = path.canonicalize()?;

    #[cfg(windows)]
    let path_buf = Path::new(
        path_buf
            .as_path()
            .to_string_lossy()
            .trim_start_matches(r"\\?\"),
    )
    .to_path_buf();

    Ok(path_buf)
}

fn get_usage() -> String {
    format!("{0} [OPTION]... FILE...", executable!())
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let usage = get_usage();

    let matches = App::new(executable!())
        .version(VERSION)
        .about(ABOUT)
        .usage(&usage[..])
        .arg(
            Arg::with_name(OPT_LOGICAL)
                .short("L")
                .long(OPT_LOGICAL)
                .help("use PWD from environment, even if it contains symlinks"),
        )
        .arg(
            Arg::with_name(OPT_PHYSICAL)
                .short("P")
                .long(OPT_PHYSICAL)
                .help("avoid all symlinks"),
        )
        .get_matches_from(args);

    match env::current_dir() {
        Ok(logical_path) => {
            if matches.is_present(OPT_LOGICAL) {
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

    0
}
