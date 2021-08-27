//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Alex Lyon <arcterus@mail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) ENOTDIR

#[macro_use]
extern crate uucore;

use clap::{crate_version, App, Arg};
use std::fs::{read_dir, remove_dir};
use std::io;
use std::path::Path;
use uucore::error::{set_exit_code, strip_errno, UResult};
use uucore::util_name;

static ABOUT: &str = "Remove the DIRECTORY(ies), if they are empty.";
static OPT_IGNORE_FAIL_NON_EMPTY: &str = "ignore-fail-on-non-empty";
static OPT_PARENTS: &str = "parents";
static OPT_VERBOSE: &str = "verbose";

static ARG_DIRS: &str = "dirs";

fn usage() -> String {
    format!("{0} [OPTION]... DIRECTORY...", uucore::execution_phrase())
}

#[uucore_procs::gen_uumain]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let usage = usage();

    let matches = uu_app().usage(&usage[..]).get_matches_from(args);

    let opts = Opts {
        ignore: matches.is_present(OPT_IGNORE_FAIL_NON_EMPTY),
        parents: matches.is_present(OPT_PARENTS),
        verbose: matches.is_present(OPT_VERBOSE),
    };

    for path in matches
        .values_of_os(ARG_DIRS)
        .unwrap_or_default()
        .map(Path::new)
    {
        if let Err(error) = remove(path, opts) {
            let Error { error, path } = error;

            if opts.ignore && dir_not_empty(&error, path) {
                continue;
            }

            set_exit_code(1);

            // If `foo` is a symlink to a directory then `rmdir foo/` may give
            // a "not a directory" error. This is confusing as `rm foo/` says
            // "is a directory".
            // This differs from system to system. Some don't give an error.
            // Windows simply allows calling RemoveDirectory on symlinks so we
            // don't need to worry about it here.
            // GNU rmdir seems to print "Symbolic link not followed" if:
            // - It has a trailing slash
            // - It's a symlink
            // - It either points to a directory or dangles
            #[cfg(unix)]
            {
                use std::ffi::OsStr;
                use std::os::unix::ffi::OsStrExt;

                fn is_symlink(path: &Path) -> io::Result<bool> {
                    Ok(path.symlink_metadata()?.file_type().is_symlink())
                }

                fn points_to_directory(path: &Path) -> io::Result<bool> {
                    Ok(path.metadata()?.file_type().is_dir())
                }

                let path = path.as_os_str().as_bytes();
                if error.raw_os_error() == Some(libc::ENOTDIR) && path.ends_with(b"/") {
                    // Strip the trailing slash or .symlink_metadata() will follow the symlink
                    let path: &Path = OsStr::from_bytes(&path[..path.len() - 1]).as_ref();
                    if is_symlink(path).unwrap_or(false)
                        && points_to_directory(path).unwrap_or(true)
                    {
                        show_error!(
                            "failed to remove '{}/': Symbolic link not followed",
                            path.display()
                        );
                        continue;
                    }
                }
            }

            show_error!(
                "failed to remove '{}': {}",
                path.display(),
                strip_errno(&error)
            );
        }
    }

    Ok(())
}

struct Error<'a> {
    error: io::Error,
    path: &'a Path,
}

fn remove(mut path: &Path, opts: Opts) -> Result<(), Error<'_>> {
    remove_single(path, opts)?;
    if opts.parents {
        while let Some(new) = path.parent() {
            path = new;
            if path.as_os_str() == "" {
                break;
            }
            remove_single(path, opts)?;
        }
    }
    Ok(())
}

fn remove_single(path: &Path, opts: Opts) -> Result<(), Error<'_>> {
    if opts.verbose {
        println!("{}: removing directory, '{}'", util_name(), path.display());
    }
    remove_dir(path).map_err(|error| Error { error, path })
}

// POSIX: https://pubs.opengroup.org/onlinepubs/009696799/functions/rmdir.html
#[cfg(not(windows))]
const NOT_EMPTY_CODES: &[i32] = &[libc::ENOTEMPTY, libc::EEXIST];

// 145 is ERROR_DIR_NOT_EMPTY, determined experimentally.
#[cfg(windows)]
const NOT_EMPTY_CODES: &[i32] = &[145];

// Other error codes you might get for directories that could be found and are
// not empty.
// This is a subset of the error codes listed in rmdir(2) from the Linux man-pages
// project. Maybe other systems have additional codes that apply?
#[cfg(not(windows))]
const PERHAPS_EMPTY_CODES: &[i32] = &[libc::EACCES, libc::EBUSY, libc::EPERM, libc::EROFS];

// Probably incomplete, I can't find a list of possible errors for
// RemoveDirectory anywhere.
#[cfg(windows)]
const PERHAPS_EMPTY_CODES: &[i32] = &[
    5, // ERROR_ACCESS_DENIED, found experimentally.
];

fn dir_not_empty(error: &io::Error, path: &Path) -> bool {
    if let Some(code) = error.raw_os_error() {
        if NOT_EMPTY_CODES.contains(&code) {
            return true;
        }
        // If --ignore-fail-on-non-empty is used then we want to ignore all errors
        // for non-empty directories, even if the error was e.g. because there's
        // no permission. So we do an additional check.
        if PERHAPS_EMPTY_CODES.contains(&code) {
            if let Ok(mut iterator) = read_dir(path) {
                if iterator.next().is_some() {
                    return true;
                }
            }
        }
    }
    false
}

#[derive(Clone, Copy, Debug)]
struct Opts {
    ignore: bool,
    parents: bool,
    verbose: bool,
}

pub fn uu_app() -> App<'static, 'static> {
    App::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
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
}
