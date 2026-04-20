// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use super::{CatResult, FdReadable, InputHandle};

use std::io::{Read, Write};
use std::os::{fd::AsFd, unix::io::AsRawFd};

use uucore::pipes::{MAX_ROOTLESS_PIPE_SIZE, might_fuse, pipe, splice, splice_exact};

/// This function is called from `write_fast()` on Linux and Android. The
/// function `splice()` is used to move data between two file descriptors
/// without copying between kernel and user spaces. This results in a large
/// speedup.
///
/// The `bool` in the result value indicates if we need to fall back to normal
/// copying or not. False means we don't have to.
#[inline]
pub(super) fn write_fast_using_splice<R: FdReadable, S: AsRawFd + AsFd + Write>(
    handle: &InputHandle<R>,
    write_fd: &mut S,
) -> CatResult<bool> {
    use std::{fs::File, sync::OnceLock};
    static PIPE_CACHE: OnceLock<Option<(File, File)>> = OnceLock::new();
    if splice(&handle.reader, &write_fd, MAX_ROOTLESS_PIPE_SIZE).is_ok() {
        // fcntl improves throughput
        // todo: avoid fcntl overhead for small input, but don't fcntl inside of the loop
        let _ = rustix::pipe::fcntl_setpipe_size(&mut *write_fd, MAX_ROOTLESS_PIPE_SIZE);
        loop {
            match splice(&handle.reader, &write_fd, MAX_ROOTLESS_PIPE_SIZE) {
                Ok(1..) => {}
                Ok(0) => return Ok(might_fuse(&handle.reader)),
                Err(_) => return Ok(true),
            }
        }
    } else if let Some((pipe_rd, pipe_wr)) = PIPE_CACHE.get_or_init(|| pipe().ok()).as_ref() {
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
                        let mut drain = Vec::with_capacity(n); // bounded by pipe size
                        pipe_rd.take(n as u64).read_to_end(&mut drain)?;
                        write_fd.write_all(&drain)?;
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
