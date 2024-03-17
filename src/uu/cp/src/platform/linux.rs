// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore ficlone reflink ftruncate pwrite fiemap
use std::fs::{File, OpenOptions};
use std::io::Read;
use std::os::unix::fs::OpenOptionsExt;
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
    let blocks: usize = src_file.metadata()?.blocks().try_into().unwrap();

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

fn check_for_seekhole(blocks: usize, size: usize) -> bool {
    // cp uses a crude heureustic for hole detection
    // an estimated formula which closely replicates GNU behavior is no of blocks < st_size/512
    // reference: https://doc.rust-lang.org/std/os/unix/fs/trait.MetadataExt.html#tymethod.blocks
    blocks < (size / 512)
}

fn check_for_non_null_element(
    source: &Path,
    non_null_flag: &mut bool,
    size_flag: &mut bool,
    sparse_val: &mut SparseDebug,
) -> std::io::Result<()> {
    //from testing GNU cp behaviour , any sparse file with non null byte , yields copy_offload:
    //avoided in the debug result and any file size < 512 yields the same.
    let mut f = File::open(source)?;

    use std::os::unix::prelude::MetadataExt;

    let size: usize = f.metadata()?.size().try_into().unwrap();
    let block_size: usize = f.metadata()?.blksize().try_into().unwrap();
    let blocks: usize = f.metadata()?.blocks().try_into().unwrap();
    if check_for_seekhole(blocks, size) {
        *sparse_val = SparseDebug::SeekHole;
    } else if size < 512 {
        *size_flag = false;
    }
    let mut buf: Vec<u8> = vec![0; block_size];
    let mut current_offset = 0;
    while current_offset < size {
        let this_read = f.read(&mut buf)?;
        if buf.iter().any(|&x| x != 0x0) {
            *non_null_flag = true;
            return Ok(());
        }

        current_offset += this_read;
    }
    Ok(())
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

    let mut size_flag = true; // size > 512
    let mut non_null_flag = false; // contains non_null_byte
    let result = match (reflink_mode, sparse_mode) {
        (ReflinkMode::Never, SparseMode::Always) => {
            let mut sparse_val = SparseDebug::Zeros; //Default sparse_debug val
            let _ = check_for_non_null_element(
                source,
                &mut non_null_flag,
                &mut size_flag,
                &mut sparse_val,
            );
            match (size_flag, non_null_flag) {
                (false, _) => {
                    copy_debug.sparse_detection = sparse_val;
                    copy_debug.offload = OffloadReflinkDebug::Avoided;
                }
                (true, false) => {
                    copy_debug.sparse_detection = sparse_val;
                    copy_debug.offload = OffloadReflinkDebug::Unknown;
                }
                (true, true) => {
                    copy_debug.sparse_detection = sparse_val;
                    copy_debug.offload = OffloadReflinkDebug::Avoided;
                }
            };
            copy_debug.reflink = OffloadReflinkDebug::No;
            sparse_copy(source, dest)
        }

        (ReflinkMode::Never, _) => {
            let mut sparse_val = SparseDebug::No;
            let _ = check_for_non_null_element(
                source,
                &mut non_null_flag,
                &mut size_flag,
                &mut sparse_val,
            );
            match (size_flag, non_null_flag) {
                (false, _) => {
                    copy_debug.sparse_detection = sparse_val;
                    copy_debug.offload = OffloadReflinkDebug::Avoided;
                }
                (true, false) => {
                    copy_debug.sparse_detection = sparse_val;
                    copy_debug.offload = OffloadReflinkDebug::Unknown;
                }
                (true, true) => {
                    copy_debug.sparse_detection = sparse_val;
                    copy_debug.offload = OffloadReflinkDebug::Avoided;
                }
            };
            copy_debug.reflink = OffloadReflinkDebug::No;
            std::fs::copy(source, dest).map(|_| ())
        }
        (ReflinkMode::Auto, SparseMode::Always) => {
            let mut sparse_val = SparseDebug::Zeros;
            let _ = check_for_non_null_element(
                source,
                &mut non_null_flag,
                &mut size_flag,
                &mut sparse_val,
            );
            match (size_flag, non_null_flag) {
                (false, _) => {
                    copy_debug.sparse_detection = sparse_val;
                    copy_debug.offload = OffloadReflinkDebug::Avoided;
                }
                (true, false) => {
                    copy_debug.sparse_detection = sparse_val;
                    copy_debug.offload = OffloadReflinkDebug::Unknown;
                }
                (true, true) => {
                    match sparse_val {
                        SparseDebug::SeekHole => sparse_val = SparseDebug::SeekHoleZeros,
                        _ => sparse_val = SparseDebug::Zeros,
                    };
                    copy_debug.sparse_detection = sparse_val;
                    copy_debug.offload = OffloadReflinkDebug::Avoided;
                }
            };

            copy_debug.reflink = OffloadReflinkDebug::Unsupported;
            sparse_copy(source, dest)
        }

        (ReflinkMode::Auto, SparseMode::Auto) => {
            copy_debug.reflink = OffloadReflinkDebug::Unsupported;
            let mut sparse_val = SparseDebug::No;
            let _ = check_for_non_null_element(
                source,
                &mut non_null_flag,
                &mut size_flag,
                &mut sparse_val,
            );
            match (size_flag, non_null_flag) {
                (false, _) => {
                    copy_debug.sparse_detection = sparse_val;
                    copy_debug.offload = OffloadReflinkDebug::Yes;
                }
                (true, false) => {
                    copy_debug.sparse_detection = sparse_val;
                    copy_debug.offload = OffloadReflinkDebug::Unknown;
                }
                (true, true) => {
                    copy_debug.sparse_detection = sparse_val;
                    copy_debug.offload = OffloadReflinkDebug::Yes;
                }
            };

            if source_is_fifo {
                copy_fifo_contents(source, dest).map(|_| ())
            } else {
                clone(source, dest, CloneFallback::FSCopy)
            }
        }
        (ReflinkMode::Auto, SparseMode::Never) => {
            copy_debug.reflink = OffloadReflinkDebug::No;
            let mut sparse_val = SparseDebug::No;
            let _ = check_for_non_null_element(
                source,
                &mut non_null_flag,
                &mut size_flag,
                &mut sparse_val,
            );
            match (size_flag, non_null_flag) {
                (false, _) => {
                    copy_debug.sparse_detection = sparse_val;
                    copy_debug.offload = OffloadReflinkDebug::Avoided;
                }
                (true, false) => {
                    copy_debug.sparse_detection = sparse_val;
                    copy_debug.offload = OffloadReflinkDebug::Unknown;
                }
                (true, true) => {
                    copy_debug.sparse_detection = sparse_val;
                    copy_debug.offload = OffloadReflinkDebug::Avoided;
                }
            };

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
