//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Haitao Li <lihaitao@gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) errno

#[macro_use]
extern crate uucore;

use std::fs;
use std::io::{stdout, Write};
use std::path::{Path, PathBuf};
use uucore::fs::{canonicalize, CanonicalizeMode};

use crate::app::*;

pub mod app;

fn get_usage() -> String {
    format!("{0} [OPTION]... [FILE]...", executable!())
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let usage = get_usage();
    let matches = get_app(executable!())
        .usage(&usage[..])
        .get_matches_from(args);

    let mut no_newline = matches.is_present(OPT_NO_NEWLINE);
    let use_zero = matches.is_present(OPT_ZERO);
    let silent = matches.is_present(OPT_SILENT) || matches.is_present(OPT_QUIET);
    let verbose = matches.is_present(OPT_VERBOSE);

    let can_mode = if matches.is_present(OPT_CANONICALIZE) {
        CanonicalizeMode::Normal
    } else if matches.is_present(OPT_CANONICALIZE_EXISTING) {
        CanonicalizeMode::Existing
    } else if matches.is_present(OPT_CANONICALIZE_MISSING) {
        CanonicalizeMode::Missing
    } else {
        CanonicalizeMode::None
    };

    let files: Vec<String> = matches
        .values_of(ARG_FILES)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_default();
    if files.is_empty() {
        crash!(
            1,
            "missing operand\nTry {} --help for more information",
            executable!()
        );
    }

    if no_newline && files.len() > 1 && !silent {
        eprintln!(
            "{}: ignoring --no-newline with multiple arguments",
            executable!()
        );
        no_newline = false;
    }

    for f in &files {
        let p = PathBuf::from(f);
        if can_mode == CanonicalizeMode::None {
            match fs::read_link(&p) {
                Ok(path) => show(&path, no_newline, use_zero),
                Err(err) => {
                    if verbose {
                        eprintln!(
                            "{}: {}: errno {}",
                            executable!(),
                            f,
                            err.raw_os_error().unwrap()
                        );
                    }
                    return 1;
                }
            }
        } else {
            match canonicalize(&p, can_mode) {
                Ok(path) => show(&path, no_newline, use_zero),
                Err(err) => {
                    if verbose {
                        eprintln!(
                            "{}: {}: errno {:?}",
                            executable!(),
                            f,
                            err.raw_os_error().unwrap()
                        );
                    }
                    return 1;
                }
            }
        }
    }

    0
}

fn show(path: &Path, no_newline: bool, use_zero: bool) {
    let path = path.to_str().unwrap();
    if use_zero {
        print!("{}\0", path);
    } else if no_newline {
        print!("{}", path);
    } else {
        println!("{}", path);
    }
    crash_if_err!(1, stdout().flush());
}
