// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use super::{CatResult, FdReadable, InputHandle};

use std::io::Write;
use std::os::{fd::AsFd, unix::io::AsRawFd};

use uucore::pipes::{MAX_ROOTLESS_PIPE_SIZE, might_fuse, splice};

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
    if splice(&handle.reader, &write_fd, MAX_ROOTLESS_PIPE_SIZE).is_ok() {
        Ok(
            uucore::pipes::splice_unbounded(&handle.reader, write_fd)?
                || might_fuse(&handle.reader),
        )
    } else {
        Ok(
            uucore::pipes::splice_unbounded_broker(&handle.reader, write_fd)?
                || might_fuse(&handle.reader),
        )
    }
}
