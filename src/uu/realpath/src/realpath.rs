// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) retcode

use clap::ArgMatches;
use std::{
    io::{stdout, Write},
    path::{Path, PathBuf},
};
use uucore::fs::make_path_relative_to;
use uucore::{
    display::{print_verbatim, Quotable},
    error::{FromIo, UClapError, UResult},
    fs::{canonicalize, MissingHandling, ResolveMode},
    line_ending::LineEnding,
    show_if_err,
};

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = crate::uu_app()
        .try_get_matches_from(args)
        .with_exit_code(1)?;

    /*  the list of files */

    let paths: Vec<PathBuf> = matches
        .get_many::<String>(crate::options::ARG_FILES)
        .unwrap()
        .map(PathBuf::from)
        .collect();

    let strip = matches.get_flag(crate::options::OPT_STRIP);
    let line_ending = LineEnding::from_zero_flag(matches.get_flag(crate::options::OPT_ZERO));
    let quiet = matches.get_flag(crate::options::OPT_QUIET);
    let logical = matches.get_flag(crate::options::OPT_LOGICAL);
    let can_mode = if matches.get_flag(crate::options::OPT_CANONICALIZE_EXISTING) {
        MissingHandling::Existing
    } else if matches.get_flag(crate::options::OPT_CANONICALIZE_MISSING) {
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
            line_ending,
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

/// Prepare `--relative-to` and `--relative-base` options.
/// Convert them to their absolute values.
/// Check if `--relative-to` is a descendant of `--relative-base`,
/// otherwise nullify their value.
fn prepare_relative_options(
    matches: &ArgMatches,
    can_mode: MissingHandling,
    resolve_mode: ResolveMode,
) -> UResult<(Option<PathBuf>, Option<PathBuf>)> {
    let relative_to = matches
        .get_one::<String>(crate::options::OPT_RELATIVE_TO)
        .map(PathBuf::from);
    let relative_base = matches
        .get_one::<String>(crate::options::OPT_RELATIVE_BASE)
        .map(PathBuf::from);
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
    line_ending: LineEnding,
    resolve: ResolveMode,
    can_mode: MissingHandling,
    relative_to: Option<&Path>,
    relative_base: Option<&Path>,
) -> std::io::Result<()> {
    let abs = canonicalize(p, can_mode, resolve)?;

    let abs = process_relative(abs, relative_base, relative_to);

    print_verbatim(abs)?;
    stdout().write_all(&[line_ending.into()])?;
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
