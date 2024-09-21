// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Thin pipe-related wrappers around functions from the `nix` crate.

use std::fs::File;
#[cfg(any(target_os = "linux", target_os = "android"))]
use std::io::IoSlice;
#[cfg(any(target_os = "linux", target_os = "android"))]
use std::os::fd::AsFd;

#[cfg(any(target_os = "linux", target_os = "android"))]
use nix::fcntl::SpliceFFlags;

pub use nix::{Error, Result};

/// A wrapper around [`nix::unistd::pipe`] that ensures the pipe is cleaned up.
///
/// Returns two `File` objects: everything written to the second can be read
/// from the first.
pub fn pipe() -> Result<(File, File)> {
    let (read, write) = nix::unistd::pipe()?;
    Ok((File::from(read), File::from(write)))
}

/// Less noisy wrapper around [`nix::fcntl::splice`].
///
/// Up to `len` bytes are moved from `source` to `target`. Returns the number
/// of successfully moved bytes.
///
/// At least one of `source` and `target` must be some sort of pipe.
/// To get around this requirement, consider splicing from your source into
/// a [`pipe`] and then from the pipe into your target (with `splice_exact`):
/// this is still very efficient.
#[cfg(any(target_os = "linux", target_os = "android"))]
pub fn splice(source: &impl AsFd, target: &impl AsFd, len: usize) -> Result<usize> {
    nix::fcntl::splice(source, None, target, None, len, SpliceFFlags::empty())
}

/// Splice wrapper which fully finishes the write.
///
/// Exactly `len` bytes are moved from `source` into `target`.
///
/// Panics if `source` runs out of data before `len` bytes have been moved.
#[cfg(any(target_os = "linux", target_os = "android"))]
pub fn splice_exact(source: &impl AsFd, target: &impl AsFd, len: usize) -> Result<()> {
    let mut left = len;
    while left != 0 {
        let written = splice(source, target, left)?;
        assert_ne!(written, 0, "unexpected end of data");
        left -= written;
    }
    Ok(())
}

/// Copy data from `bytes` into `target`, which must be a pipe.
///
/// Returns the number of successfully copied bytes.
#[cfg(any(target_os = "linux", target_os = "android"))]
pub fn vmsplice(target: &impl AsFd, bytes: &[u8]) -> Result<usize> {
    nix::fcntl::vmsplice(target, &[IoSlice::new(bytes)], SpliceFFlags::empty())
}
