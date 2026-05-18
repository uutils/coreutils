// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Buffer-based copying implementation for Linux and Android.

use crate::error::UResult;

/// Buffer-based copying utilities for unix (excluding Linux).
use std::{
    io::{Read, Write},
    os::fd::{AsFd, AsRawFd},
};

/// A readable file descriptor.
pub trait FdReadable: Read + AsRawFd + AsFd {}

impl<T> FdReadable for T where T: Read + AsFd + AsRawFd {}

/// A writable file descriptor.
pub trait FdWritable: Write + AsFd + AsRawFd {}

impl<T> FdWritable for T where T: Write + AsFd + AsRawFd {}

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
pub fn copy_stream<R, S>(src: &mut R, dest: &mut S) -> UResult<()>
where
    R: Read + AsFd + AsRawFd,
    S: Write + AsFd + AsRawFd,
{
    // try to use the splice() system call
    // for faster writing. If it works, we're done
    if crate::pipes::splice_unbounded_auto(&src, dest)? {
        std::io::copy(src, dest)?;
        // todo: Do not mix writing by raw syscall and std's buffered write,
        // or order of output would be wrong when this was called multiple times
        // and splice_unbounded_auto sent content partially. flush works as an workaround.
        dest.flush()?;
    }
    Ok(())
}
