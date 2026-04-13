// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Buffer-based copying implementation for Linux and Android.

use crate::{
    error::UResult,
    pipes::{copy_exact, pipe, splice, splice_exact},
};

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

const SPLICE_SIZE: usize = 1024 * 128;

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
///
/// # Returns
///
/// Result of operation and bytes successfully written (as a `u64`) when
/// operation is successful.
pub fn copy_stream<R, S>(src: &mut R, dest: &mut S) -> UResult<u64>
where
    R: Read + AsFd + AsRawFd,
    S: Write + AsFd + AsRawFd,
{
    // If we're on Linux or Android, try to use the splice() system call
    // for faster writing. If it works, we're done.
    let result = splice_write(src, &dest.as_fd())?;
    if !result.1 {
        return Ok(result.0);
    }

    // If the splice() call failed, fall back on slower writing.
    let result = std::io::copy(src, dest)?;

    // If the splice() call failed and there has been some data written to
    // stdout via while loop above AND there will be second splice() call
    // that will succeed, data pushed through splice will be output before
    // the data buffered in stdout.lock. Therefore additional explicit flush
    // is required here.
    dest.flush()?;
    Ok(result)
}

/// Write from source `handle` into destination `write_fd` using Linux-specific
/// `splice` system call.
///
/// # Arguments
/// - `source` - source handle
/// - `dest` - destination handle
#[inline]
pub(crate) fn splice_write<R, S>(source: &R, dest: &S) -> UResult<(u64, bool)>
where
    R: Read + AsFd + AsRawFd,
    S: AsRawFd + AsFd,
{
    let (pipe_rd, pipe_wr) = pipe()?;
    let mut bytes: u64 = 0;

    loop {
        match splice(&source, &pipe_wr, SPLICE_SIZE) {
            Ok(n) => {
                if n == 0 {
                    return Ok((bytes, false));
                }
                if splice_exact(&pipe_rd, dest, n).is_err() {
                    // If the first splice manages to copy to the intermediate
                    // pipe, but the second splice to stdout fails for some reason
                    // we can recover by copying the data that we have from the
                    // intermediate pipe to stdout using normal read/write. Then
                    // we tell the caller to fall back.
                    copy_exact(&pipe_rd, dest, n)?;
                    return Ok((bytes, true));
                }

                bytes += n as u64;
            }
            Err(_) => {
                return Ok((bytes, true));
            }
        }
    }
}
