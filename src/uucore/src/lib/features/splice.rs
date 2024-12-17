// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! This module provides the [`write_fast_using_splice`] function to leverage the `splice` system call
//! in Linux systems, thus increasing the I/O performance of copying between two file descriptors.

use nix::unistd;
use std::{
    io::Read,
    os::{
        fd::AsFd,
        unix::io::{AsRawFd, RawFd},
    },
};

#[cfg(any(target_os = "linux", target_os = "android"))]
use crate::pipes::{pipe, splice, splice_exact};

const SPLICE_SIZE: usize = 1024 * 128;
const BUF_SIZE: usize = 1024 * 16;

/// `splice` is a Linux-specific system call used to move data between two file descriptors without
/// copying between kernel and user spaces. This results in a large speedup.
///
/// This function reads from a file/stream `handle` and directly writes to `write_fd`. Returns the
/// amount of bytes written as a `u64`.
///
/// The `bool` in the result value indicates if we need to fall back to normal
/// copying or not. False means we don't have to.
#[inline]
pub fn write_fast_using_splice<R: Read + AsFd + AsRawFd, S: AsRawFd + AsFd>(
    handle: &R,
    write_fd: &S,
) -> nix::Result<(usize, bool)> {
    let (pipe_rd, pipe_wr) = pipe()?;
    let mut bytes = 0;

    loop {
        match splice(&handle, &pipe_wr, SPLICE_SIZE) {
            Ok(n) => {
                if n == 0 {
                    return Ok((bytes, false));
                }
                if splice_exact(&pipe_rd, write_fd, n).is_err() {
                    // If the first splice manages to copy to the intermediate
                    // pipe, but the second splice to stdout fails for some reason
                    // we can recover by copying the data that we have from the
                    // intermediate pipe to stdout using normal read/write. Then
                    // we tell the caller to fall back.
                    copy_exact(pipe_rd.as_raw_fd(), write_fd, n)?;
                    return Ok((bytes, true));
                }

                bytes += n;
            }
            Err(_) => {
                return Ok((bytes, true));
            }
        }
    }
}

/// Move exactly `num_bytes` bytes from `read_fd` to `write_fd`.
///
/// Panics if not enough bytes can be read.
fn copy_exact(read_fd: RawFd, write_fd: &impl AsFd, num_bytes: usize) -> nix::Result<()> {
    let mut left = num_bytes;
    let mut buf = [0; BUF_SIZE];
    while left > 0 {
        let read = unistd::read(read_fd, &mut buf)?;
        assert_ne!(read, 0, "unexpected end of pipe");
        let mut written = 0;
        while written < read {
            match unistd::write(write_fd, &buf[written..read])? {
                0 => panic!(),
                n => written += n,
            }
        }
        left -= read;
    }
    Ok(())
}
