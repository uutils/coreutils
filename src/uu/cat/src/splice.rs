// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use super::{CatResult, FdReadable, InputHandle};

use rustix::io::{read, write};
use std::os::{fd::AsFd, unix::io::AsRawFd};

use uucore::pipes::{MAX_ROOTLESS_PIPE_SIZE, might_fuse, pipe, splice, splice_exact};

const BUF_SIZE: usize = 1024 * 16;

/// This function is called from `write_fast()` on Linux and Android. The
/// function `splice()` is used to move data between two file descriptors
/// without copying between kernel and user spaces. This results in a large
/// speedup.
///
/// The `bool` in the result value indicates if we need to fall back to normal
/// copying or not. False means we don't have to.
#[inline]
pub(super) fn write_fast_using_splice<R: FdReadable, S: AsRawFd + AsFd>(
    handle: &InputHandle<R>,
    write_fd: &S,
) -> CatResult<bool> {
    if splice(&handle.reader, &write_fd, MAX_ROOTLESS_PIPE_SIZE).is_ok() {
        // fcntl improves throughput
        // todo: avoid fcntl overhead for small input, but don't fcntl inside of the loop
        let _ = rustix::pipe::fcntl_setpipe_size(write_fd, MAX_ROOTLESS_PIPE_SIZE);
        loop {
            match splice(&handle.reader, &write_fd, MAX_ROOTLESS_PIPE_SIZE) {
                Ok(1..) => {}
                Ok(0) => return Ok(might_fuse(&handle.reader)),
                Err(_) => return Ok(true),
            }
        }
    } else if let Ok((pipe_rd, pipe_wr)) = pipe() {
        // both of in/output are not pipe. needs broker to use splice() with additional costs
        loop {
            match splice(&handle.reader, &pipe_wr, MAX_ROOTLESS_PIPE_SIZE) {
                Ok(0) => return Ok(might_fuse(&handle.reader)),
                Ok(n) => {
                    if splice_exact(&pipe_rd, write_fd, n).is_err() {
                        // If the first splice manages to copy to the intermediate
                        // pipe, but the second splice to stdout fails for some reason
                        // we can recover by copying the data that we have from the
                        // intermediate pipe to stdout using normal read/write. Then
                        // we tell the caller to fall back.
                        copy_exact(&pipe_rd, write_fd, n)?;
                        return Ok(true);
                    }
                }
                Err(_) => return Ok(true),
            }
        }
    } else {
        Ok(true)
    }
}

/// Move exactly `num_bytes` bytes from `read_fd` to `write_fd`.
///
/// Panics if not enough bytes can be read.
fn copy_exact(read_fd: &impl AsFd, write_fd: &impl AsFd, num_bytes: usize) -> std::io::Result<()> {
    let mut left = num_bytes;
    let mut buf = [0; BUF_SIZE];
    while left > 0 {
        let n = read(read_fd, &mut buf)?;
        assert_ne!(n, 0, "unexpected end of pipe");
        let mut written = 0;
        while written < n {
            match write(write_fd, &buf[written..n])? {
                0 => unreachable!("fd should be writable"),
                w => written += w,
            }
        }
        left -= n;
    }
    Ok(())
}
