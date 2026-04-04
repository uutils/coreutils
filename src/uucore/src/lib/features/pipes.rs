// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Thin zero-copy-related wrappers around functions from the `rustix` crate.

#[cfg(any(target_os = "linux", target_os = "android"))]
use rustix::pipe::{SpliceFlags, fcntl_setpipe_size};
#[cfg(any(target_os = "linux", target_os = "android"))]
use std::fs::File;
#[cfg(any(target_os = "linux", target_os = "android"))]
use std::os::fd::AsFd;
pub const MAX_ROOTLESS_PIPE_SIZE: usize = 1024 * 1024;

/// A wrapper around [`rustix::pipe::pipe`] that ensures the pipe is cleaned up.
///
/// Returns two `File` objects: everything written to the second can be read
/// from the first.
/// This is used only for resolving the limitation for splice: one of a input or output should be pipe
#[inline]
#[cfg(any(target_os = "linux", target_os = "android"))]
pub fn pipe() -> std::io::Result<(File, File)> {
    let (read, write) = rustix::pipe::pipe()?;
    // improve performance for splice
    let _ = fcntl_setpipe_size(&read, MAX_ROOTLESS_PIPE_SIZE);

    Ok((File::from(read), File::from(write)))
}

/// Less noisy wrapper around [`rustix::pipe::splice`].
///
/// Up to `len` bytes are moved from `source` to `target`. Returns the number
/// of successfully moved bytes.
///
/// At least one of `source` and `target` must be some sort of pipe.
/// To get around this requirement, consider splicing from your source into
/// a [`pipe`] and then from the pipe into your target (with `splice_exact`):
/// this is still very efficient.
#[inline]
#[cfg(any(target_os = "linux", target_os = "android"))]
pub fn splice(source: &impl AsFd, target: &impl AsFd, len: usize) -> std::io::Result<usize> {
    Ok(rustix::pipe::splice(
        source,
        None,
        target,
        None,
        len,
        SpliceFlags::empty(),
    )?)
}

/// Splice wrapper which fully finishes the write.
///
/// Exactly `len` bytes are moved from `source` into `target`.
///
/// Panics if `source` runs out of data before `len` bytes have been moved.
#[inline]
#[cfg(any(target_os = "linux", target_os = "android"))]
pub fn splice_exact(source: &impl AsFd, target: &impl AsFd, len: usize) -> std::io::Result<()> {
    let mut left = len;
    while left > 0 {
        let written = splice(source, target, left)?;
        debug_assert_ne!(written, 0, "unexpected end of data");
        left -= written;
    }
    Ok(())
}

/// Return verified /dev/null
///
/// `splice` to /dev/null is faster than `read` when we skip or count the input which is not able to seek
#[inline]
#[cfg(any(target_os = "linux", target_os = "android"))]
pub fn dev_null() -> Option<File> {
    let null = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/null")
        .ok()?;
    let stat = rustix::fs::fstat(&null).ok()?;
    let dev = stat.st_rdev;
    if (rustix::fs::major(dev), rustix::fs::minor(dev)) == (1, 3) {
        Some(null)
    } else {
        None
    }
}
