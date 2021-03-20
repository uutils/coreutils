//  * This file is part of the uutils coreutils package.
//  *
//  * (c) 2014 Vsevolod Velichko <torkvemada@sorokdva.net>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) retcode

#[macro_use]
extern crate uucore;

use clap::{App, Arg};
use std::fs;
use std::path::PathBuf;
use uucore::fs::{canonicalize, CanonicalizeMode};

static ABOUT: &str = "print the resolved path";
static VERSION: &str = env!("CARGO_PKG_VERSION");

static OPT_QUIET: &str = "quiet";
static OPT_STRIP: &str = "strip";
static OPT_ZERO: &str = "zero";

static ARG_FILES: &str = "files";

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
            Arg::with_name(OPT_QUIET)
                .short("q")
                .long(OPT_QUIET)
                .help("Do not print warnings for invalid paths"),
        )
        .arg(
            Arg::with_name(OPT_STRIP)
                .short("s")
                .long(OPT_STRIP)
                .help("Only strip '.' and '..' components, but don't resolve symbolic links"),
        )
        .arg(
            Arg::with_name(OPT_ZERO)
                .short("z")
                .long(OPT_ZERO)
                .help("Separate output filenames with \\0 rather than newline"),
        )
        .arg(
            Arg::with_name(ARG_FILES)
                .multiple(true)
                .takes_value(true)
                .required(true)
                .min_values(1),
        )
        .get_matches_from(args);

    /*  the list of files */

    let paths: Vec<PathBuf> = matches
        .values_of(ARG_FILES)
        .unwrap()
        .map(PathBuf::from)
        .collect();

    let strip = matches.is_present(OPT_STRIP);
    let zero = matches.is_present(OPT_ZERO);
    let quiet = matches.is_present(OPT_QUIET);
    let mut retcode = 0;
    for path in &paths {
        if !resolve_path(path, strip, zero, quiet) {
            retcode = 1
        };
    }
    retcode
}

fn resolve_path(p: &PathBuf, strip: bool, zero: bool, quiet: bool) -> bool {
    let abs = canonicalize(p, CanonicalizeMode::Normal).unwrap();

    if strip {
        if zero {
            print!("{}\0", p.display());
        } else {
            println!("{}", p.display())
        }
        return true;
    }

    let mut result = PathBuf::new();
    let mut links_left = 256;

    for part in abs.components() {
        result.push(part.as_os_str());
        loop {
            if links_left == 0 {
                if !quiet {
                    show_error!("Too many symbolic links: {}", p.display())
                };
                return false;
            }
            match fs::metadata(result.as_path()) {
                Err(_) => break,
                Ok(ref m) if !m.file_type().is_symlink() => break,
                Ok(_) => {
                    links_left -= 1;
                    match fs::read_link(result.as_path()) {
                        Ok(x) => {
                            result.pop();
                            result.push(x.as_path());
                        }
                        _ => {
                            if !quiet {
                                show_error!("Invalid path: {}", p.display())
                            };
                            return false;
                        }
                    }
                }
            }
        }
    }

    if zero {
        print!("{}\0", result.display());
    } else {
        println!("{}", result.display());
    }

    true
}
