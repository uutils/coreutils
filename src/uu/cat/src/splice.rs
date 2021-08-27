use super::{CatResult, FdReadable, InputHandle};

use nix::unistd;
use std::os::unix::io::{AsRawFd, RawFd};

use uucore::pipes::{pipe, splice, splice_exact};

const BUF_SIZE: usize = 1024 * 16;

/// This function is called from `write_fast()` on Linux and Android. The
/// function `splice()` is used to move data between two file descriptors
/// without copying between kernel and user spaces. This results in a large
/// speedup.
///
/// The `bool` in the result value indicates if we need to fall back to normal
/// copying or not. False means we don't have to.
#[inline]
pub(super) fn write_fast_using_splice<R: FdReadable>(
    handle: &mut InputHandle<R>,
    write_fd: &impl AsRawFd,
) -> CatResult<bool> {
    let (pipe_rd, pipe_wr) = pipe()?;

    loop {
        match splice(&handle.reader, &pipe_wr, BUF_SIZE) {
            Ok(n) => {
                if n == 0 {
                    return Ok(false);
                }
                if splice_exact(&pipe_rd, write_fd, n).is_err() {
                    // If the first splice manages to copy to the intermediate
                    // pipe, but the second splice to stdout fails for some reason
                    // we can recover by copying the data that we have from the
                    // intermediate pipe to stdout using normal read/write. Then
                    // we tell the caller to fall back.
                    copy_exact(pipe_rd.as_raw_fd(), write_fd.as_raw_fd(), n)?;
                    return Ok(true);
                }
            }
            Err(_) => {
                return Ok(true);
            }
        }
    }
}

/// Caller must ensure that `num_bytes <= BUF_SIZE`, otherwise this function
/// will panic. The way we use this function in `write_fast_using_splice`
/// above is safe because `splice` is set to write at most `BUF_SIZE` to the
/// pipe.
#[inline]
fn copy_exact(read_fd: RawFd, write_fd: RawFd, num_bytes: usize) -> nix::Result<()> {
    let mut left = num_bytes;
    let mut buf = [0; BUF_SIZE];
    loop {
        let read = unistd::read(read_fd, &mut buf[..left])?;
        let written = unistd::write(write_fd, &buf[..read])?;
        left -= written;
        if left == 0 {
            break;
        }
    }
    Ok(())
}
