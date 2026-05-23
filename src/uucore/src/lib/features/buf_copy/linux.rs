// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Buffer-based copying implementation for Linux and Android.
use std::os::fd::AsFd;

/// Copy data from `Read` implementor `source` into a `Write` implementor
/// `dest`. This works by reading a chunk of data from `source` and writing the
/// data to `dest` in a loop.
///
/// This function uses the Linux-specific `splice` call when possible which does
/// not use any intermediate user-space buffer. It falls backs to
/// `std::io::copy` when the call fails and is still recoverable.
///
/// # Arguments
/// * `source` - `Read` implementor to copy data from.
/// * `dest` - `Write` implementor to copy data to.
pub fn copy_stream(
    src: &mut (impl std::io::Read + AsFd),
    dest: &mut impl AsFd,
) -> crate::error::UResult<()> {
    // If we're on Linux or Android, try to use the splice() system call
    // for faster writing. If it works, we're done.
    if crate::pipes::splice_unbounded_auto(src, dest)?.is_err() {
        // If the splice() call failed, fall back on writing "without buffering", or order of output would be wrong
        // unrelated for cp /dev/stdin since cp does not have multiple input? <https://github.com/uutils/coreutils/issues/5186>
        // RawWriter also removes io::copy's specialization
        std::io::copy(src, &mut crate::io::RawWriter(dest))?;
    }
    Ok(())
}
