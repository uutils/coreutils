//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Derek Chiang <derekchiang93@gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// clippy bug https://github.com/rust-lang/rust-clippy/issues/7422
#![allow(clippy::nonstandard_macro_braces)]

#[macro_use]
extern crate uucore;

use clap::{crate_version, App, Arg};
use std::env;
use std::io;
use std::path::{Path, PathBuf};

use uucore::error::{UResult, USimpleError};

static ABOUT: &str = "Display the full filename of the current working directory.";
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

#[uucore_procs::gen_uumain]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let usage = get_usage();

    let matches = uu_app().usage(&usage[..]).get_matches_from(args);

    match env::current_dir() {
        Ok(logical_path) => {
            if matches.is_present(OPT_LOGICAL) {
                println!("{}", logical_path.display());
            } else {
                match absolute_path(&logical_path) {
                    Ok(physical_path) => println!("{}", physical_path.display()),
                    Err(e) => {
                        return Err(USimpleError::new(
                            1,
                            format!("failed to get absolute path {}", e),
                        ))
                    }
                };
            }
        }
        Err(e) => {
            return Err(USimpleError::new(
                1,
                format!("failed to get current directory {}", e),
            ))
        }
    };

    Ok(())
}

pub fn uu_app() -> App<'static, 'static> {
    App::new(executable!())
        .version(crate_version!())
        .about(ABOUT)
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
}
