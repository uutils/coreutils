//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Derek Chiang <derekchiang93@gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

#[macro_use]
extern crate uucore;

use std::env;
use std::io;
use std::path::{Path, PathBuf};

use crate::app::get_app;
use crate::app::OPT_LOGICAL;

pub mod app;

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

    let matches = get_app(executable!())
        .usage(&usage[..])
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
