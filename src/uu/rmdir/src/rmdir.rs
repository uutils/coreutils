//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Alex Lyon <arcterus@mail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

#[macro_use]
extern crate uucore;

use clap::{App, Arg};
use std::fs;
use std::path::Path;

static VERSION: &str = env!("CARGO_PKG_VERSION");
static ABOUT: &str = "Remove the DIRECTORY(ies), if they are empty.";
static OPT_IGNORE_FAIL_NON_EMPTY: &str = "ignore-fail-on-non-empty";
static OPT_PARENTS: &str = "parents";
static OPT_VERBOSE: &str = "verbose";

static ARG_DIRS: &str = "dirs";

fn get_usage() -> String {
    format!("{0} [OPTION]... DIRECTORY...", executable!())
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let usage = get_usage();

    let matches = App::new(executable!())
        .version(VERSION)
        .about(ABOUT)
        .usage(&usage[..])
        .arg(
            Arg::with_name(OPT_IGNORE_FAIL_NON_EMPTY)
                .long(OPT_IGNORE_FAIL_NON_EMPTY)
                .help("ignore each failure that is solely because a directory is non-empty"),
        )
        .arg(
            Arg::with_name(OPT_PARENTS)
                .short("p")
                .long(OPT_PARENTS)
                .help(
                    "remove DIRECTORY and its ancestors; e.g.,
                  'rmdir -p a/b/c' is similar to rmdir a/b/c a/b a",
                ),
        )
        .arg(
            Arg::with_name(OPT_VERBOSE)
                .short("v")
                .long(OPT_VERBOSE)
                .help("output a diagnostic for every directory processed"),
        )
        .arg(
            Arg::with_name(ARG_DIRS)
                .multiple(true)
                .takes_value(true)
                .min_values(1)
                .required(true),
        )
        .get_matches_from(args);

    let dirs: Vec<String> = matches
        .values_of(ARG_DIRS)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_default();

    let ignore = matches.is_present(OPT_IGNORE_FAIL_NON_EMPTY);
    let parents = matches.is_present(OPT_PARENTS);
    let verbose = matches.is_present(OPT_VERBOSE);

    match remove(dirs, ignore, parents, verbose) {
        Ok(()) => ( /* pass */ ),
        Err(e) => return e,
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
            }
            Ok(_) if verbose => println!("removing directory, '{}'", path.display()),
            _ => (),
        }
    } else if !ignore {
        show_error!("failed to remove '{}': Directory not empty", path.display());
        r = Err(1);
    }

    r
}
