//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Haitao Li <lihaitao@gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) errno

extern crate clap;

#[macro_use]
extern crate uucore;

use clap::{App, Arg};
use std::fs;
use std::io::{stdout, Write};
use std::path::PathBuf;
use uucore::fs::{canonicalize, CanonicalizeMode};

const NAME: &str = "readlink";
static ABOUT: &str = "Print value of a symbolic link or canonical file name";
const VERSION: &str = env!("CARGO_PKG_VERSION");
const OPT_CANONICALIZE: &str = "canonicalize";
const OPT_CANONICALIZE_EXISTING: &str = "canonicalize-existing";
const OPT_CANONICALIZE_MISSING: &str = "canonicalize-missing";
const OPT_FILES: &str = "files";

fn get_usage() -> String {
    format!("{0} [OPTION]... [FILE]...", executable!())
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let usage = get_usage();
    let matches = App::new(executable!())
        .version(VERSION)
        .about(ABOUT)
        .usage(&usage[..])
        .arg(
            Arg::with_name(OPT_CANONICALIZE)
                .short("f")
                .long(OPT_CANONICALIZE)
                .help(
                    "canonicalize by following every symlink in every component of the \
                given name recursively; all but the last component must exist",
                ),
        )
        .arg(
            Arg::with_name(OPT_CANONICALIZE_EXISTING)
                .short("e")
                .long(OPT_CANONICALIZE_EXISTING)
                .help(
                    "canonicalize by following every symlink in every component of the \
                given name recursively, all components must exist",
                ),
        )
        .arg(
            Arg::with_name(OPT_CANONICALIZE_MISSING)
                .short("m")
                .long("canonicalize-missing")
                .help(
                    "canonicalize by following every symlink in every component of the \
                given name recursively, without requirements on components existence",
                ),
        )
        .arg(
            Arg::with_name("n")
                .short("n")
                .long("no-newline")
                .help("do not output the trailing delimiter"),
        )
        .arg(
            Arg::with_name("q")
                .short("q")
                .long("quiet")
                .help("suppress most error messages"),
        )
        .arg(
            Arg::with_name("s")
                .short("s")
                .long("silent")
                .help("suppress most error messages"),
        )
        .arg(
            Arg::with_name("v")
                .short("v")
                .long("verbose")
                .help("report error message"),
        )
        .arg(
            Arg::with_name("z")
                .short("z")
                .long("zero")
                .help("separate output with NUL rather than newline"),
        )
        .arg(Arg::with_name(OPT_FILES).multiple(true).takes_value(true))
        .get_matches_from(args);

    let mut no_newline = matches.is_present("no-newline");
    let use_zero = matches.is_present("zero");
    let silent = matches.is_present("silent") || matches.is_present("quiet");
    let verbose = matches.is_present("verbose");

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
        .values_of(OPT_FILES)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_default();

    if files.is_empty() {
        crash!(
            1,
            "missing operand\nTry {} --help for more information",
            NAME
        );
    }

    if no_newline && files.len() > 1 && !silent {
        eprintln!("{}: ignoring --no-newline with multiple arguments", NAME);
        no_newline = false;
    }

    for f in &files {
        let p = PathBuf::from(f);
        if can_mode == CanonicalizeMode::None {
            match fs::read_link(&p) {
                Ok(path) => show(&path, no_newline, use_zero),
                Err(err) => {
                    if verbose {
                        eprintln!("{}: {}: errno {}", NAME, f, err.raw_os_error().unwrap());
                    }
                    return 1;
                }
            }
        } else {
            match canonicalize(&p, can_mode) {
                Ok(path) => show(&path, no_newline, use_zero),
                Err(err) => {
                    if verbose {
                        eprintln!("{}: {}: errno {:?}", NAME, f, err.raw_os_error().unwrap());
                    }
                    return 1;
                }
            }
        }
    }

    0
}

fn show(path: &PathBuf, no_newline: bool, use_zero: bool) {
    let path = path.as_path().to_str().unwrap();
    if use_zero {
        print!("{}\0", path);
    } else if no_newline {
        print!("{}", path);
    } else {
        println!("{}", path);
    }
    crash_if_err!(1, stdout().flush());
}
