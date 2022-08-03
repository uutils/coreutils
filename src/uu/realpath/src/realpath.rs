//  * This file is part of the uutils coreutils package.
//  *
//  * (c) 2014 Vsevolod Velichko <torkvemada@sorokdva.net>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) retcode

#[macro_use]
extern crate uucore;

use clap::{crate_version, Arg, ArgMatches, Command};
use std::{
    io::{stdout, Write},
    path::{Path, PathBuf},
};
use uucore::error::UClapError;
use uucore::fs::make_path_relative_to;
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
const OPT_RELATIVE_TO: &str = "relative-to";
const OPT_RELATIVE_BASE: &str = "relative-base";

static ARG_FILES: &str = "files";

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().try_get_matches_from(args).with_exit_code(1)?;

    /*  the list of files */

    let paths: Vec<PathBuf> = matches
        .values_of(ARG_FILES)
        .unwrap()
        .map(PathBuf::from)
        .collect();

    let strip = matches.contains_id(OPT_STRIP);
    let zero = matches.contains_id(OPT_ZERO);
    let quiet = matches.contains_id(OPT_QUIET);
    let logical = matches.contains_id(OPT_LOGICAL);
    let can_mode = if matches.contains_id(OPT_CANONICALIZE_EXISTING) {
        MissingHandling::Existing
    } else if matches.contains_id(OPT_CANONICALIZE_MISSING) {
        MissingHandling::Missing
    } else {
        MissingHandling::Normal
    };
    let resolve_mode = if strip {
        ResolveMode::None
    } else if logical {
        ResolveMode::Logical
    } else {
        ResolveMode::Physical
    };
    let (relative_to, relative_base) = prepare_relative_options(&matches, can_mode, resolve_mode)?;
    for path in &paths {
        let result = resolve_path(
            path,
            zero,
            resolve_mode,
            can_mode,
            relative_to.as_deref(),
            relative_base.as_deref(),
        );
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
                .visible_alias("no-symlinks")
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
            Arg::new(OPT_RELATIVE_TO)
                .long(OPT_RELATIVE_TO)
                .takes_value(true)
                .value_name("DIR")
                .forbid_empty_values(true)
                .help("print the resolved path relative to DIR"),
        )
        .arg(
            Arg::new(OPT_RELATIVE_BASE)
                .long(OPT_RELATIVE_BASE)
                .takes_value(true)
                .value_name("DIR")
                .forbid_empty_values(true)
                .help("print absolute paths unless paths below DIR"),
        )
        .arg(
            Arg::new(ARG_FILES)
                .multiple_occurrences(true)
                .required(true)
                .forbid_empty_values(true)
                .value_hint(clap::ValueHint::AnyPath),
        )
}

/// Prepare `--relative-to` and `--relative-base` options.
/// Convert them to their absolute values.
/// Check if `--relative-to` is a descendant of `--relative-base`,
/// otherwise nullify their value.
fn prepare_relative_options(
    matches: &ArgMatches,
    can_mode: MissingHandling,
    resolve_mode: ResolveMode,
) -> UResult<(Option<PathBuf>, Option<PathBuf>)> {
    let relative_to = matches.value_of(OPT_RELATIVE_TO).map(PathBuf::from);
    let relative_base = matches.value_of(OPT_RELATIVE_BASE).map(PathBuf::from);
    let relative_to = canonicalize_relative_option(relative_to, can_mode, resolve_mode)?;
    let relative_base = canonicalize_relative_option(relative_base, can_mode, resolve_mode)?;
    if let (Some(base), Some(to)) = (relative_base.as_deref(), relative_to.as_deref()) {
        if !to.starts_with(base) {
            return Ok((None, None));
        }
    }
    Ok((relative_to, relative_base))
}

/// Prepare single `relative-*` option.
fn canonicalize_relative_option(
    relative: Option<PathBuf>,
    can_mode: MissingHandling,
    resolve_mode: ResolveMode,
) -> UResult<Option<PathBuf>> {
    Ok(match relative {
        None => None,
        Some(p) => Some(
            canonicalize_relative(&p, can_mode, resolve_mode)
                .map_err_context(|| p.maybe_quote().to_string())?,
        ),
    })
}

/// Make `relative-to` or `relative-base` path values absolute.
///
/// # Errors
///
/// If the given path is not a directory the function returns an error.
/// If some parts of the file don't exist, or symlinks make loops, or
/// some other IO error happens, the function returns error, too.
fn canonicalize_relative(
    r: &Path,
    can_mode: MissingHandling,
    resolve: ResolveMode,
) -> std::io::Result<PathBuf> {
    let abs = canonicalize(r, can_mode, resolve)?;
    if can_mode == MissingHandling::Existing && !abs.is_dir() {
        abs.read_dir()?; // raise not a directory error
    }
    Ok(abs)
}

/// Resolve a path to an absolute form and print it.
///
/// If `relative_to` and/or `relative_base` is given
/// the path is printed in a relative form to one of this options.
/// See the details in `process_relative` function.
/// If `zero` is `true`, then this function
/// prints the path followed by the null byte (`'\0'`) instead of a
/// newline character (`'\n'`).
///
/// # Errors
///
/// This function returns an error if there is a problem resolving
/// symbolic links.
fn resolve_path(
    p: &Path,
    zero: bool,
    resolve: ResolveMode,
    can_mode: MissingHandling,
    relative_to: Option<&Path>,
    relative_base: Option<&Path>,
) -> std::io::Result<()> {
    let abs = canonicalize(p, can_mode, resolve)?;
    let line_ending = if zero { b'\0' } else { b'\n' };

    let abs = process_relative(abs, relative_base, relative_to);

    print_verbatim(&abs)?;
    stdout().write_all(&[line_ending])?;
    Ok(())
}

/// Conditionally converts an absolute path to a relative form,
/// according to the rules:
/// 1. if only `relative_to` is given, the result is relative to `relative_to`
/// 1. if only `relative_base` is given, it checks whether given `path` is a descendant
/// of `relative_base`, on success the result is relative to `relative_base`, otherwise
/// the result is the given `path`
/// 1. if both `relative_to` and `relative_base` are given, the result is relative to `relative_to`
/// if `path` is a descendant of `relative_base`, otherwise the result is `path`
///
/// For more information see
/// <https://www.gnu.org/software/coreutils/manual/html_node/Realpath-usage-examples.html>
fn process_relative(
    path: PathBuf,
    relative_base: Option<&Path>,
    relative_to: Option<&Path>,
) -> PathBuf {
    if let Some(base) = relative_base {
        if path.starts_with(base) {
            make_path_relative_to(path, relative_to.unwrap_or(base))
        } else {
            path
        }
    } else if let Some(to) = relative_to {
        make_path_relative_to(path, to)
    } else {
        path
    }
}
