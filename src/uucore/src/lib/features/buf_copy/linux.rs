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

use super::common::Error;

/// A readable file descriptor.
pub trait FdReadable: Read + AsRawFd + AsFd {}

impl<T> FdReadable for T where T: Read + AsFd + AsRawFd {}

/// A writable file descriptor.
pub trait FdWritable: Write + AsFd + AsRawFd {}

impl<T> FdWritable for T where T: Write + AsFd + AsRawFd {}

/// Conversion from a `rustix::io::Errno` into our `Error` which implements `UError`.
impl From<rustix::io::Errno> for Error {
    fn from(error: rustix::io::Errno) -> Self {
        Self::Io(std::io::Error::from(error))
    }
}

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
    // If we're on Linux or Android, try to use the splice() system call
    // for faster writing. If it works, we're done.
    // todo: bypass broker pipe this if input or output is pipe. We use this mostly for stream.
    if !crate::pipes::splice_unbounded_broker(src, dest)? {
        return Ok(());
    }

    // If the splice() call failed, fall back on slower writing.
    std::io::copy(src, dest)?;

    // If the splice() call failed and there has been some data written to
    // stdout via while loop above AND there will be second splice() call
    // that will succeed, data pushed through splice will be output before
    // the data buffered in stdout.lock. Therefore additional explicit flush
    // is required here.
    dest.flush()?;
    Ok(())
}
