// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use super::{CatResult, FdReadable, InputHandle};

use std::os::fd::AsFd;

use uucore::pipes::{MAX_ROOTLESS_PIPE_SIZE, might_fuse, splice};

/// This function is called from `write_fast()` on Linux and Android. The
/// function `splice()` is used to move data between two file descriptors
/// without copying between kernel and user spaces. This results in a large
/// speedup.
///
/// The `SpliceState` in the result value indicates if we need to fall back to
/// normal copying or not. `SpliceState::Ended` means we don't have to.
#[inline]
pub(super) fn write_fast_using_splice<R: FdReadable, S: AsFd>(
    handle: &InputHandle<R>,
    write_fd: &mut S,
) -> CatResult<uucore::pipes::SpliceState> {
    let splice_state = match splice(&handle.reader, &write_fd, MAX_ROOTLESS_PIPE_SIZE) {
        Ok(_) => uucore::pipes::splice_unbounded(&handle.reader, write_fd)?,
        // both of in/output are not pipe
        _ => uucore::pipes::splice_unbounded_broker(&handle.reader, write_fd)?,
    };
    let final_state = match splice_state {
        uucore::pipes::SpliceState::Ended if might_fuse(&handle.reader) => {
            uucore::pipes::SpliceState::Fallback
        }
        state => state,
    };
    Ok(final_state)
}
