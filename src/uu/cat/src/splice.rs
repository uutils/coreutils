// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use super::{CatResult, FdReadable, InputHandle};

use std::os::fd::AsFd;

/// This function is called from `write_fast()` on Linux and Android. The
/// function `splice()` is used to move data between two file descriptors
/// without copying between kernel and user spaces. This results in a large
/// speedup.
///
/// The `bool` in the result value indicates if we need to fall back to normal
/// copying or not. False means we don't have to.
#[inline]
pub(super) fn write_fast_using_splice<R: FdReadable, S: AsFd>(
    handle: &InputHandle<R>,
    write_fd: &mut S,
) -> CatResult<bool> {
    let res = uucore::pipes::splice_unbounded_auto(&handle.reader, write_fd)?;
    Ok(res || uucore::pipes::might_fuse(&handle.reader))
}
