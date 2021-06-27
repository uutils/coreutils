//  * This file is part of the uutils coreutils package.
//  *
//  * (c) 2014 Vsevolod Velichko <torkvemada@sorokdva.net>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) retcode

#[macro_use]
extern crate uucore;

use clap::{crate_version, App, Arg};
use std::path::{Component, Path, PathBuf};
use uucore::fs::{canonicalize, MissingHandling, ResolveMode};

static ABOUT: &str = "print the resolved path";

static OPT_QUIET: &str = "quiet";
static OPT_STRIP: &str = "strip";
static OPT_ZERO: &str = "zero";
static OPT_PHYSICAL: &str = "physical";
static OPT_LOGICAL: &str = "logical";

static ARG_FILES: &str = "files";

fn get_usage() -> String {
    format!("{0} [OPTION]... FILE...", executable!())
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let usage = get_usage();

    let matches = uu_app().usage(&usage[..]).get_matches_from(args);

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
    let mut retcode = 0;
    for path in &paths {
        if let Err(e) = resolve_path(path, strip, zero, logical) {
            if !quiet {
                show_error!("{}: {}", e, path.display());
            }
            retcode = 1
        };
    }
    retcode
}

pub fn uu_app() -> App<'static, 'static> {
    App::new(executable!())
        .version(crate_version!())
        .about(ABOUT)
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
            Arg::with_name(OPT_LOGICAL)
                .short("L")
                .long(OPT_LOGICAL)
                .help("resolve '..' components before symlinks"),
        )
        .arg(
            Arg::with_name(OPT_PHYSICAL)
                .short("P")
                .long(OPT_PHYSICAL)
                .overrides_with_all(&[OPT_STRIP, OPT_LOGICAL])
                .help("resolve symlinks as encountered (default)"),
        )
        .arg(
            Arg::with_name(ARG_FILES)
                .multiple(true)
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
fn resolve_path(p: &Path, strip: bool, zero: bool, logical: bool) -> std::io::Result<()> {
    let resolve = if strip {
        ResolveMode::None
    } else if logical {
        ResolveMode::Logical
    } else {
        ResolveMode::Physical
    };
    let abs = canonicalize(p, MissingHandling::Normal, resolve)?;
    let line_ending = if zero { '\0' } else { '\n' };

    let mut final_path = PathBuf::new();
    if cfg!(windows) && resolve == ResolveMode::Physical {
        // strip the '\\?\' prefix to the sting indicating 'Extended Length Path syntax' (https://docs.microsoft.com/en-us/windows/win32/fileio/naming-a-file#namespaces)
        for part in abs.components() {
            match part {
                Component::Prefix(_) => {
                    final_path.push(&part.as_os_str().to_str().unwrap()[4..]);
                }
                _ => {
                    final_path.push(part);
                }
            }
        }
    } else {
        final_path = abs;
    }

    print!("{}{}", final_path.display(), line_ending);
    Ok(())
}
