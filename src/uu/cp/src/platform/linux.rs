//  * This file is part of the uutils coreutils package.
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.
// spell-checker:ignore ficlone reflink ftruncate pwrite
use std::fs::File;
use std::io::Read;
use std::os::unix::io::AsRawFd;
use std::path::Path;

use quick_error::ResultExt;

use crate::{CopyResult, ReflinkMode, SparseMode};

// From /usr/include/linux/fs.h:
// #define FICLONE		_IOW(0x94, 9, int)
// Use a macro as libc::ioctl expects u32 or u64 depending on the arch
macro_rules! FICLONE {
    () => {
        0x40049409
    };
}

/// Use the Linux `ioctl_ficlone` API to do a copy-on-write clone.
///
/// If `fallback` is true and there is a failure performing the clone,
/// then this function performs a standard [`std::fs::copy`]. Otherwise,
/// this function returns an error.
#[cfg(any(target_os = "linux", target_os = "android"))]
fn clone<P>(source: P, dest: P, fallback: bool) -> std::io::Result<()>
where
    P: AsRef<Path>,
{
    let src_file = File::open(&source)?;
    let dst_file = File::create(&dest)?;
    let src_fd = src_file.as_raw_fd();
    let dst_fd = dst_file.as_raw_fd();
    let result = unsafe { libc::ioctl(dst_fd, FICLONE!(), src_fd) };
    if result != 0 {
        if fallback {
            std::fs::copy(source, dest).map(|_| ())
        } else {
            Err(std::io::Error::last_os_error())
        }
    } else {
        Ok(())
    }
}

/// Perform a sparse copy from one file to another.
#[cfg(any(target_os = "linux", target_os = "android"))]
fn sparse_copy<P>(source: P, dest: P) -> std::io::Result<()>
where
    P: AsRef<Path>,
{
    use std::os::unix::prelude::MetadataExt;

    let mut src_file = File::open(source)?;
    let dst_file = File::create(dest)?;
    let dst_fd = dst_file.as_raw_fd();

    let size: usize = src_file.metadata()?.size().try_into().unwrap();
    if unsafe { libc::ftruncate(dst_fd, size.try_into().unwrap()) } < 0 {
        return Err(std::io::Error::last_os_error());
    }

    let blksize = dst_file.metadata()?.blksize();
    let mut buf: Vec<u8> = vec![0; blksize.try_into().unwrap()];
    let mut current_offset: usize = 0;

    // TODO Perhaps we can employ the "fiemap ioctl" API to get the
    // file extent mappings:
    // https://www.kernel.org/doc/html/latest/filesystems/fiemap.html
    while current_offset < size {
        let this_read = src_file.read(&mut buf)?;
        if buf.iter().any(|&x| x != 0) {
            unsafe {
                libc::pwrite(
                    dst_fd,
                    buf.as_ptr() as *const libc::c_void,
                    this_read,
                    current_offset.try_into().unwrap(),
                )
            };
        }
        current_offset += this_read;
    }
    Ok(())
}

/// Copies `source` to `dest` using copy-on-write if possible.
pub(crate) fn copy_on_write(
    source: &Path,
    dest: &Path,
    reflink_mode: ReflinkMode,
    sparse_mode: SparseMode,
    context: &str,
) -> CopyResult<()> {
    let result = match (reflink_mode, sparse_mode) {
        (ReflinkMode::Never, _) => std::fs::copy(source, dest).map(|_| ()),
        (ReflinkMode::Auto, SparseMode::Always) => sparse_copy(source, dest),
        (ReflinkMode::Auto, _) => clone(source, dest, true),
        (ReflinkMode::Always, SparseMode::Auto) => clone(source, dest, false),
        (ReflinkMode::Always, _) => {
            return Err("`--reflink=always` can be used only with --sparse=auto".into())
        }
    };
    result.context(context)?;
    Ok(())
}
