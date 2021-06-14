//  * This file is part of the uutils coreutils package.
//  *
//  * (c) 2014 Vsevolod Velichko <torkvemada@sorokdva.net>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) retcode

#[macro_use]
extern crate uucore;

use std::path::{Path, PathBuf};
use uucore::fs::{canonicalize, CanonicalizeMode};

use crate::app::*;

pub mod app;

fn get_usage() -> String {
    format!("{0} [OPTION]... FILE...", executable!())
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let usage = get_usage();

    let matches = get_app(executable!())
        .usage(&usage[..])
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
        if let Err(e) = resolve_path(path, strip, zero) {
            if !quiet {
                show_error!("{}: {}", e, path.display());
            }
            retcode = 1
        };
    }
    retcode
}

/// Resolve a path to an absolute form and print it.
///
/// If `strip` is `true`, then this function does not attempt to resolve
/// symbolic links in the path. If `zero` is `true`, then this function
/// prints the path followed by the null byte (`'\0'`) instead of a
/// newline character (`'\n'`).
///
/// # Errors
///
/// This function returns an error if there is a problem resolving
/// symbolic links.
fn resolve_path(p: &Path, strip: bool, zero: bool) -> std::io::Result<()> {
    let mode = if strip {
        CanonicalizeMode::None
    } else {
        CanonicalizeMode::Normal
    };
    let abs = canonicalize(p, mode)?;
    let line_ending = if zero { '\0' } else { '\n' };
    print!("{}{}", abs.display(), line_ending);
    Ok(())
}
