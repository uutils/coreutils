// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Check if a file is ordered.
//!
//! On most platforms this uses a multi-threaded reader. On WASI without
//! atomics, a synchronous variant is used instead. The two implementations
//! live in sibling modules and are selected via cfg at the module boundary.

use std::cmp::Ordering;
use std::ffi::OsStr;

use uucore::error::UResult;

use crate::{GlobalSettings, open};

#[cfg(not(wasi_no_threads))]
mod threaded;
#[cfg(not(wasi_no_threads))]
use threaded as runner;

#[cfg(wasi_no_threads)]
mod sync;
#[cfg(wasi_no_threads)]
use sync as runner;

/// Check if the file at `path` is ordered.
pub fn check(path: &OsStr, settings: &GlobalSettings) -> UResult<()> {
    let max_allowed_cmp = if settings.unique {
        Ordering::Less
    } else {
        Ordering::Equal
    };
    let file = open(path)?;
    let chunk_size = if settings.buffer_size < 100 * 1024 {
        settings.buffer_size
    } else {
        100 * 1024
    };

    runner::check(path, settings, max_allowed_cmp, file, chunk_size)
}
