// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) ENOTDIR

use uucore::show_error;

use std::ffi::OsString;
use std::fs::{read_dir, remove_dir};
use std::io;
use std::path::Path;
use uucore::display::Quotable;
use uucore::error::{set_exit_code, strip_errno, UResult};

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = crate::uu_app().try_get_matches_from(args)?;

    let opts = Opts {
        ignore: matches.get_flag(crate::options::OPT_IGNORE_FAIL_NON_EMPTY),
        parents: matches.get_flag(crate::options::OPT_PARENTS),
        verbose: matches.get_flag(crate::options::OPT_VERBOSE),
    };

    for path in matches
        .get_many::<OsString>(crate::options::ARG_DIRS)
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

                fn points_to_directory(path: &Path) -> io::Result<bool> {
                    Ok(path.metadata()?.file_type().is_dir())
                }

                let bytes = path.as_os_str().as_bytes();
                if error.raw_os_error() == Some(libc::ENOTDIR) && bytes.ends_with(b"/") {
                    // Strip the trailing slash or .symlink_metadata() will follow the symlink
                    let no_slash: &Path = OsStr::from_bytes(&bytes[..bytes.len() - 1]).as_ref();
                    if no_slash.is_symlink() && points_to_directory(no_slash).unwrap_or(true) {
                        show_error!(
                            "failed to remove {}: Symbolic link not followed",
                            path.quote()
                        );
                        continue;
                    }
                }
            }

            show_error!("failed to remove {}: {}", path.quote(), strip_errno(&error));
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
            if path.as_os_str().is_empty() {
                break;
            }
            remove_single(path, opts)?;
        }
    }
    Ok(())
}

fn remove_single(path: &Path, opts: Opts) -> Result<(), Error<'_>> {
    if opts.verbose {
        println!(
            "{}: removing directory, {}",
            uucore::util_name(),
            path.quote()
        );
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
