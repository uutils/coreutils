//  * This file is part of the uutils coreutils package.
//  *
//  * (c) 2014 Vsevolod Velichko <torkvemada@sorokdva.net>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) retcode

#[macro_use]
extern crate uucore;

use clap::{crate_version, Arg, Command};
use std::{
    io::{stdout, Write},
    path::{Path, PathBuf},
};
use uucore::{
    display::{print_verbatim, Quotable},
    error::{FromIo, UResult},
    format_usage,
    fs::{canonicalize, MissingHandling, ResolveMode},
};

static ABOUT: &str = "print the resolved path";
const USAGE: &str = "{} [OPTION]... FILE...";

static OPT_QUIET: &str = "quiet";
static OPT_STRIP: &str = "strip";
static OPT_ZERO: &str = "zero";
static OPT_PHYSICAL: &str = "physical";
static OPT_LOGICAL: &str = "logical";
const OPT_CANONICALIZE_MISSING: &str = "canonicalize-missing";
const OPT_CANONICALIZE_EXISTING: &str = "canonicalize-existing";

static ARG_FILES: &str = "files";

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().get_matches_from(args);

    /*  the list of files */

    let paths: Vec<PathBuf> = matches
        .values_of(ARG_FILES)
        .unwrap()
        .map(PathBuf::from)
        .collect();

    let strip = matches.is_present(OPT_STRIP);
    let zero = matches.is_present(OPT_ZERO);
    let quiet = matches.is_present(OPT_QUIET);
    let logical = matches.is_present(OPT_LOGICAL);
    let can_mode = if matches.is_present(OPT_CANONICALIZE_EXISTING) {
        MissingHandling::Existing
    } else if matches.is_present(OPT_CANONICALIZE_MISSING) {
        MissingHandling::Missing
    } else {
        MissingHandling::Normal
    };
    for path in &paths {
        let result = resolve_path(path, strip, zero, logical, can_mode);
        if !quiet {
            show_if_err!(result.map_err_context(|| path.maybe_quote().to_string()));
        }
    }
    // Although we return `Ok`, it is possible that a call to
    // `show!()` above has set the exit code for the program to a
    // non-zero integer.
    Ok(())
}

pub fn uu_app<'a>() -> Command<'a> {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(OPT_QUIET)
                .short('q')
                .long(OPT_QUIET)
                .help("Do not print warnings for invalid paths"),
        )
        .arg(
            Arg::new(OPT_STRIP)
                .short('s')
                .long(OPT_STRIP)
                .help("Only strip '.' and '..' components, but don't resolve symbolic links"),
        )
        .arg(
            Arg::new(OPT_ZERO)
                .short('z')
                .long(OPT_ZERO)
                .help("Separate output filenames with \\0 rather than newline"),
        )
        .arg(
            Arg::new(OPT_LOGICAL)
                .short('L')
                .long(OPT_LOGICAL)
                .help("resolve '..' components before symlinks"),
        )
        .arg(
            Arg::new(OPT_PHYSICAL)
                .short('P')
                .long(OPT_PHYSICAL)
                .overrides_with_all(&[OPT_STRIP, OPT_LOGICAL])
                .help("resolve symlinks as encountered (default)"),
        )
        .arg(
            Arg::new(OPT_CANONICALIZE_EXISTING)
                .short('e')
                .long(OPT_CANONICALIZE_EXISTING)
                .help(
                    "canonicalize by following every symlink in every component of the \
                     given name recursively, all components must exist",
                ),
        )
        .arg(
            Arg::new(OPT_CANONICALIZE_MISSING)
                .short('m')
                .long(OPT_CANONICALIZE_MISSING)
                .help(
                    "canonicalize by following every symlink in every component of the \
                     given name recursively, without requirements on components existence",
                ),
        )
        .arg(
            Arg::new(ARG_FILES)
                .multiple_occurrences(true)
                .takes_value(true)
                .required(true)
                .min_values(1),
        )
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
fn resolve_path(
    p: &Path,
    strip: bool,
    zero: bool,
    logical: bool,
    can_mode: MissingHandling,
) -> std::io::Result<()> {
    let resolve = if strip {
        ResolveMode::None
    } else if logical {
        ResolveMode::Logical
    } else {
        ResolveMode::Physical
    };
    let abs = canonicalize(p, can_mode, resolve)?;
    let line_ending = if zero { b'\0' } else { b'\n' };

    print_verbatim(&abs)?;
    stdout().write_all(&[line_ending])?;
    Ok(())
}
