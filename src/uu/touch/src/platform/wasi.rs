// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) filetime utimensat

use filetime::FileTime;
use rustix::fs::{AtFlags, CWD, Timespec, Timestamps, utimensat};
use std::io::{Error, Result};
use std::path::{Path, PathBuf};

use uucore::translate;

use crate::error::TouchError;

/// WASI replacement for `filetime::set_symlink_file_times`.
///
/// The `filetime` crate has an unimplemented stub on `wasm32-wasi`. WASI
/// supports setting times via `utimensat`, which we reach through `rustix`.
/// The non-symlink path goes through `set_times_by_path` in the main module,
/// which adds a mode-0 retry, so only the symlink variant is needed here.
pub fn set_symlink_file_times(path: &Path, atime: FileTime, mtime: FileTime) -> Result<()> {
    set_times(path, atime, mtime, AtFlags::SYMLINK_NOFOLLOW)
}

fn set_times(path: &Path, atime: FileTime, mtime: FileTime, flags: AtFlags) -> Result<()> {
    let timestamps = Timestamps {
        last_access: Timespec {
            tv_sec: atime.unix_seconds(),
            tv_nsec: atime.nanoseconds() as _,
        },
        last_modification: Timespec {
            tv_sec: mtime.unix_seconds(),
            tv_nsec: mtime.nanoseconds() as _,
        },
    };
    utimensat(CWD, path, &timestamps, flags).map_err(Error::from)
}

/// WASI has no way to name the file behind stdout.
pub fn pathbuf_from_stdout() -> std::result::Result<PathBuf, TouchError> {
    Err(TouchError::UnsupportedPlatformFeature(translate!(
        "touch-error-stdout-unsupported"
    )))
}
