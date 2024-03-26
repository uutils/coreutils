// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore ficlone reflink ftruncate pwrite fiemap
use std::fs::{self, File, OpenOptions};
use std::io::Read;
use std::os::unix::fs::{FileTypeExt, OpenOptionsExt};
use std::os::unix::io::AsRawFd;
use std::path::Path;

use quick_error::ResultExt;

use uucore::mode::get_umask;

use crate::{CopyDebug, CopyResult, OffloadReflinkDebug, ReflinkMode, SparseDebug, SparseMode};

// From /usr/include/linux/fs.h:
// #define FICLONE		_IOW(0x94, 9, int)
// Use a macro as libc::ioctl expects u32 or u64 depending on the arch
macro_rules! FICLONE {
    () => {
        0x40049409
    };
}

/// The fallback behavior for [`clone`] on failed system call.
#[derive(Clone, Copy)]
enum CloneFallback {
    /// Raise an error.
    Error,

    /// Use [`std::fs::copy`].
    FSCopy,

    /// Use sparse_copy
    SparseCopy,
}

/// Use the Linux `ioctl_ficlone` API to do a copy-on-write clone.
///
/// `fallback` controls what to do if the system call fails.
#[cfg(any(target_os = "linux", target_os = "android"))]
fn clone<P>(source: P, dest: P, fallback: CloneFallback) -> std::io::Result<()>
where
    P: AsRef<Path>,
{
    let src_file = File::open(&source)?;
    let dst_file = File::create(&dest)?;
    let src_fd = src_file.as_raw_fd();
    let dst_fd = dst_file.as_raw_fd();
    let result = unsafe { libc::ioctl(dst_fd, FICLONE!(), src_fd) };
    if result == 0 {
        return Ok(());
    }
    match fallback {
        CloneFallback::Error => Err(std::io::Error::last_os_error()),
        CloneFallback::FSCopy => std::fs::copy(source, dest).map(|_| ()),
        CloneFallback::SparseCopy => sparse_copy(source, dest),
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

/// Copy the contents of the given source FIFO to the given file.
fn copy_fifo_contents<P>(source: P, dest: P) -> std::io::Result<u64>
where
    P: AsRef<Path>,
{
    // For some reason,
    //
    //     cp --preserve=ownership --copy-contents fifo fifo2
    //
    // causes `fifo2` to be created with limited permissions (mode 622
    // or maybe 600 it seems), and then after `fifo` is closed, the
    // permissions get updated to match those of `fifo`. This doesn't
    // make much sense to me but the behavior appears in
    // `tests/cp/file-perm-race.sh`.
    //
    // So it seems that if `--preserve=ownership` is true then what we
    // need to do is create the destination file with limited
    // permissions, copy the contents, then update the permissions. If
    // `--preserve=ownership` is not true, however, then we can just
    // match the mode of the source file.
    //
    // TODO Update the code below to respect the case where
    // `--preserve=ownership` is not true.
    let mut src_file = File::open(&source)?;
    let mode = 0o622 & !get_umask();
    let mut dst_file = OpenOptions::new()
        .create(true)
        .write(true)
        .mode(mode)
        .open(&dest)?;
    let num_bytes_copied = std::io::copy(&mut src_file, &mut dst_file)?;
    dst_file.set_permissions(src_file.metadata()?.permissions())?;
    Ok(num_bytes_copied)
}

fn check_dest_is_fifo(dest: &Path) -> bool {
    let file_type = fs::metadata(dest);
    match file_type {
        Ok(f) => f.file_type().is_fifo(),

        _ => false,
    }
}
fn check_sparse_detection(source: &Path) -> Result<bool, std::io::Error> {
    let src_file = File::open(source)?;

    use std::os::unix::prelude::MetadataExt;

    let size: usize = src_file.metadata()?.size().try_into().unwrap();
    let blocks: usize = src_file.metadata()?.blocks().try_into().unwrap();
    if blocks < size / 512 && blocks != 0 {
        return Ok(true);
    }
    Ok(false)
}

/// Copies `source` to `dest` using copy-on-write if possible.
///
/// The `source_is_fifo` flag must be set to `true` if and only if
/// `source` is a FIFO (also known as a named pipe). In this case,
/// copy-on-write is not possible, so we copy the contents using
/// [`std::io::copy`].
pub(crate) fn copy_on_write(
    source: &Path,
    dest: &Path,
    reflink_mode: ReflinkMode,
    sparse_mode: SparseMode,
    context: &str,
    source_is_fifo: bool,
) -> CopyResult<CopyDebug> {
    let mut copy_debug = CopyDebug {
        offload: OffloadReflinkDebug::Unknown,
        reflink: OffloadReflinkDebug::Unsupported,
        sparse_detection: SparseDebug::No,
    };

    let result = match (reflink_mode, sparse_mode) {
        (ReflinkMode::Never, SparseMode::Always) => {
            copy_debug.sparse_detection = SparseDebug::Zeros;
            copy_debug.offload = OffloadReflinkDebug::Avoided;
            copy_debug.reflink = OffloadReflinkDebug::No;
            sparse_copy(source, dest)
        }
        (ReflinkMode::Never, SparseMode::Never) => {
            copy_debug.sparse_detection = SparseDebug::No;
            copy_debug.reflink = OffloadReflinkDebug::No;
            std::fs::copy(source, dest).map(|_| ())
        }
        (ReflinkMode::Never, SparseMode::Auto) => {
            copy_debug.sparse_detection = SparseDebug::No;
            copy_debug.reflink = OffloadReflinkDebug::No;
            match check_sparse_detection(source) {
                Ok(true) => sparse_copy(source, dest),
                Ok(false) => std::fs::copy(source, dest).map(|_| ()),
                Err(e) => Err(e),
            }
        }
        (ReflinkMode::Auto, SparseMode::Always) => {
            copy_debug.offload = OffloadReflinkDebug::Avoided;
            copy_debug.sparse_detection = SparseDebug::Zeros;
            copy_debug.reflink = OffloadReflinkDebug::Unsupported;
            sparse_copy(source, dest)
        }

        (ReflinkMode::Auto, SparseMode::Auto) => {
            copy_debug.sparse_detection = SparseDebug::No;
            copy_debug.reflink = OffloadReflinkDebug::Unsupported;
            if source_is_fifo {
                copy_fifo_contents(source, dest).map(|_| ())
            } else {
                match (check_sparse_detection(source), check_dest_is_fifo(dest)) {
                    (Ok(true), false) => clone(source, dest, CloneFallback::SparseCopy),
                    (Ok(true), true) => clone(source, dest, CloneFallback::FSCopy),
                    (Ok(false), _) => clone(source, dest, CloneFallback::FSCopy),
                    (Err(e), _) => Err(e),
                }
            }
        }
        (ReflinkMode::Auto, SparseMode::Never) => {
            copy_debug.sparse_detection = SparseDebug::No;
            copy_debug.reflink = OffloadReflinkDebug::Unsupported;
            if source_is_fifo {
                copy_fifo_contents(source, dest).map(|_| ())
            } else {
                clone(source, dest, CloneFallback::FSCopy)
            }
        }
        (ReflinkMode::Always, SparseMode::Auto) => {
            copy_debug.sparse_detection = SparseDebug::No;
            copy_debug.reflink = OffloadReflinkDebug::Yes;

            clone(source, dest, CloneFallback::Error)
        }
        (ReflinkMode::Always, _) => {
            return Err("`--reflink=always` can be used only with --sparse=auto".into())
        }
    };
    result.context(context)?;
    Ok(copy_debug)
}
