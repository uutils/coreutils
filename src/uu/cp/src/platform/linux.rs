// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore ficlone reflink ftruncate pwrite fiemap lseek

use libc::{SEEK_DATA, SEEK_HOLE};
use std::fs::{File, OpenOptions};
use std::io::Read;
use std::os::unix::fs::FileExt;
use std::os::unix::fs::MetadataExt;
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

    /// Use sparse_copy_without_hole
    SparseCopyWithoutHole,
}

/// Type of method used for copying files
#[derive(Clone, Copy)]
enum CopyMethod {
    /// Do a sparse copy
    SparseCopy,
    /// Use [`std::fs::copy`].
    FSCopy,
    /// Default (can either be sparse_copy or FSCopy)
    Default,
    /// Use sparse_copy_without_hole
    SparseCopyWithoutHole,
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
        CloneFallback::SparseCopyWithoutHole => sparse_copy_without_hole(source, dest),
    }
}

/// Checks whether a file contains any non null bytes i.e. any byte != 0x0
/// This function returns a tuple of (bool, u64, u64) signifying a tuple of (whether a file has
/// data, its size, no of blocks it has allocated in disk)
#[cfg(any(target_os = "linux", target_os = "android"))]
fn check_for_data(source: &Path) -> Result<(bool, u64, u64), std::io::Error> {
    let mut src_file = File::open(source)?;
    let metadata = src_file.metadata()?;

    let size = metadata.size();
    let blocks = metadata.blocks();
    // checks edge case of virtual files in /proc which have a size of zero but contains data
    if size == 0 {
        let mut buf: Vec<u8> = vec![0; metadata.blksize() as usize]; // Directly use metadata.blksize()
        let _ = src_file.read(&mut buf)?;
        return Ok((buf.iter().any(|&x| x != 0x0), size, 0));
    }

    let src_fd = src_file.as_raw_fd();

    let result = unsafe { libc::lseek(src_fd, 0, SEEK_DATA) };

    match result {
        -1 => Ok((false, size, blocks)), // No data found or end of file
        _ if result >= 0 => Ok((true, size, blocks)), // Data found
        _ => Err(std::io::Error::last_os_error()),
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
/// Checks whether a file is sparse i.e. it contains holes, uses the crude heuristic blocks < size / 512
/// Reference:`<https://doc.rust-lang.org/std/os/unix/fs/trait.MetadataExt.html#tymethod.blocks>`
fn check_sparse_detection(source: &Path) -> Result<bool, std::io::Error> {
    let src_file = File::open(source)?;
    let metadata = src_file.metadata()?;
    let size = metadata.size();
    let blocks = metadata.blocks();

    if blocks < size / 512 {
        return Ok(true);
    }
    Ok(false)
}

/// Optimized sparse_copy, doesn't create holes for large sequences of zeros in non sparse_files
/// Used when --sparse=auto
#[cfg(any(target_os = "linux", target_os = "android"))]
fn sparse_copy_without_hole<P>(source: P, dest: P) -> std::io::Result<()>
where
    P: AsRef<Path>,
{
    let src_file = File::open(source)?;
    let dst_file = File::create(dest)?;
    let dst_fd = dst_file.as_raw_fd();

    let size = src_file.metadata()?.size();
    if unsafe { libc::ftruncate(dst_fd, size.try_into().unwrap()) } < 0 {
        return Err(std::io::Error::last_os_error());
    }
    let src_fd = src_file.as_raw_fd();
    let mut current_offset: isize = 0;
    loop {
        let result = unsafe { libc::lseek(src_fd, current_offset.try_into().unwrap(), SEEK_DATA) }
            .try_into()
            .unwrap();

        current_offset = result;
        let hole: isize =
            unsafe { libc::lseek(src_fd, current_offset.try_into().unwrap(), SEEK_HOLE) }
                .try_into()
                .unwrap();
        if result == -1 || hole == -1 {
            break;
        }
        if result <= -2 || hole <= -2 {
            return Err(std::io::Error::last_os_error());
        }
        let len: isize = hole - current_offset;
        let mut buf: Vec<u8> = vec![0x0; len as usize];
        src_file.read_exact_at(&mut buf, current_offset as u64)?;
        unsafe {
            libc::pwrite(
                dst_fd,
                buf.as_ptr() as *const libc::c_void,
                len as usize,
                current_offset.try_into().unwrap(),
            )
        };
        current_offset = hole;
    }
    Ok(())
}
/// Perform a sparse copy from one file to another.
/// Creates a holes for large sequences of zeros in non_sparse_files, used for --sparse=always
#[cfg(any(target_os = "linux", target_os = "android"))]
fn sparse_copy<P>(source: P, dest: P) -> std::io::Result<()>
where
    P: AsRef<Path>,
{
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

#[cfg(any(target_os = "linux", target_os = "android"))]
/// Checks whether an existing destination is a fifo
fn check_dest_is_fifo(dest: &Path) -> bool {
    // If our destination file exists and its a fifo , we do a standard copy .
    let file_type = std::fs::metadata(dest);
    match file_type {
        Ok(f) => f.file_type().is_fifo(),

        _ => false,
    }
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
            // Default SparseDebug val for SparseMode::Always
            copy_debug.reflink = OffloadReflinkDebug::No;
            if source_is_fifo {
                copy_debug.offload = OffloadReflinkDebug::Avoided;

                copy_fifo_contents(source, dest).map(|_| ())
            } else {
                let mut copy_method = CopyMethod::Default;
                let result = handle_reflink_never_sparse_always(source, dest);
                if let Ok((debug, method)) = result {
                    copy_debug = debug;
                    copy_method = method;
                }

                match copy_method {
                    CopyMethod::FSCopy => std::fs::copy(source, dest).map(|_| ()),
                    _ => sparse_copy(source, dest),
                }
            }
        }
        (ReflinkMode::Never, SparseMode::Never) => {
            copy_debug.reflink = OffloadReflinkDebug::No;

            if source_is_fifo {
                copy_debug.offload = OffloadReflinkDebug::Avoided;

                copy_fifo_contents(source, dest).map(|_| ())
            } else {
                let result = handle_reflink_never_sparse_never(source);
                if let Ok(debug) = result {
                    copy_debug = debug;
                }
                std::fs::copy(source, dest).map(|_| ())
            }
        }
        (ReflinkMode::Never, SparseMode::Auto) => {
            copy_debug.reflink = OffloadReflinkDebug::No;

            if source_is_fifo {
                copy_debug.offload = OffloadReflinkDebug::Avoided;
                copy_fifo_contents(source, dest).map(|_| ())
            } else {
                let mut copy_method = CopyMethod::Default;
                let result = handle_reflink_never_sparse_auto(source, dest);
                if let Ok((debug, method)) = result {
                    copy_debug = debug;
                    copy_method = method;
                }

                match copy_method {
                    CopyMethod::SparseCopyWithoutHole => sparse_copy_without_hole(source, dest),
                    _ => std::fs::copy(source, dest).map(|_| ()),
                }
            }
        }
        (ReflinkMode::Auto, SparseMode::Always) => {
            copy_debug.sparse_detection = SparseDebug::Zeros; // Default SparseDebug val for
                                                              // SparseMode::Always
            if source_is_fifo {
                copy_debug.offload = OffloadReflinkDebug::Avoided;

                copy_fifo_contents(source, dest).map(|_| ())
            } else {
                let mut copy_method = CopyMethod::Default;
                let result = handle_reflink_auto_sparse_always(source, dest);
                if let Ok((debug, method)) = result {
                    copy_debug = debug;
                    copy_method = method;
                }

                match copy_method {
                    CopyMethod::FSCopy => clone(source, dest, CloneFallback::FSCopy),
                    _ => clone(source, dest, CloneFallback::SparseCopy),
                }
            }
        }

        (ReflinkMode::Auto, SparseMode::Never) => {
            copy_debug.reflink = OffloadReflinkDebug::No;
            if source_is_fifo {
                copy_debug.offload = OffloadReflinkDebug::Avoided;
                copy_fifo_contents(source, dest).map(|_| ())
            } else {
                let result = handle_reflink_auto_sparse_never(source);
                if let Ok(debug) = result {
                    copy_debug = debug;
                }

                clone(source, dest, CloneFallback::FSCopy)
            }
        }
        (ReflinkMode::Auto, SparseMode::Auto) => {
            if source_is_fifo {
                copy_debug.offload = OffloadReflinkDebug::Unsupported;
                copy_fifo_contents(source, dest).map(|_| ())
            } else {
                let mut copy_method = CopyMethod::Default;
                let result = handle_reflink_auto_sparse_auto(source, dest);
                if let Ok((debug, method)) = result {
                    copy_debug = debug;
                    copy_method = method;
                }

                match copy_method {
                    CopyMethod::SparseCopyWithoutHole => {
                        clone(source, dest, CloneFallback::SparseCopyWithoutHole)
                    }
                    _ => clone(source, dest, CloneFallback::FSCopy),
                }
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

/// Handles debug results when flags are "--reflink=auto" and "--sparse=always" and specifies what
/// type of copy should be used
fn handle_reflink_auto_sparse_always(
    source: &Path,
    dest: &Path,
) -> Result<(CopyDebug, CopyMethod), std::io::Error> {
    let mut copy_debug = CopyDebug {
        offload: OffloadReflinkDebug::Unknown,
        reflink: OffloadReflinkDebug::Unsupported,
        sparse_detection: SparseDebug::Zeros,
    };
    let mut copy_method = CopyMethod::Default;
    let (data_flag, size, blocks) = check_for_data(source)?;
    let sparse_flag = check_sparse_detection(source)?;

    if data_flag || size < 512 {
        copy_debug.offload = OffloadReflinkDebug::Avoided;
    }
    match (sparse_flag, data_flag, blocks) {
        (true, true, 0) => {
            // Handling funny files with 0 block allocation but has data
            // in it
            copy_method = CopyMethod::FSCopy;
            copy_debug.sparse_detection = SparseDebug::SeekHoleZeros;
        }
        (false, true, 0) => copy_method = CopyMethod::FSCopy,

        (true, false, 0) => copy_debug.sparse_detection = SparseDebug::SeekHole,
        (true, true, _) => copy_debug.sparse_detection = SparseDebug::SeekHoleZeros,

        (true, false, _) => copy_debug.sparse_detection = SparseDebug::SeekHole,

        (_, _, _) => (),
    }
    if check_dest_is_fifo(dest) {
        copy_method = CopyMethod::FSCopy;
    }
    Ok((copy_debug, copy_method))
}

/// Handles debug results when flags are "--reflink=auto" and "--sparse=auto" and specifies what
/// type of copy should be used
fn handle_reflink_never_sparse_never(source: &Path) -> Result<CopyDebug, std::io::Error> {
    let mut copy_debug = CopyDebug {
        offload: OffloadReflinkDebug::Unknown,
        reflink: OffloadReflinkDebug::No,
        sparse_detection: SparseDebug::No,
    };
    let (data_flag, size, _blocks) = check_for_data(source)?;
    let sparse_flag = check_sparse_detection(source)?;

    if sparse_flag {
        copy_debug.sparse_detection = SparseDebug::SeekHole;
    }

    if data_flag || size < 512 {
        copy_debug.offload = OffloadReflinkDebug::Avoided;
    }
    Ok(copy_debug)
}

/// Handles debug results when flags are "--reflink=auto" and "--sparse=never", files will be copied
/// through cloning them with fallback switching to std::fs::copy
fn handle_reflink_auto_sparse_never(source: &Path) -> Result<CopyDebug, std::io::Error> {
    let mut copy_debug = CopyDebug {
        offload: OffloadReflinkDebug::Unknown,
        reflink: OffloadReflinkDebug::No,
        sparse_detection: SparseDebug::No,
    };

    let (data_flag, size, _blocks) = check_for_data(source)?;
    let sparse_flag = check_sparse_detection(source)?;

    if sparse_flag {
        copy_debug.sparse_detection = SparseDebug::SeekHole;
    }

    if data_flag || size < 512 {
        copy_debug.offload = OffloadReflinkDebug::Avoided;
    }
    Ok(copy_debug)
}

/// Handles debug results when flags are "--reflink=auto" and "--sparse=auto" and specifies what
/// type of copy should be used
fn handle_reflink_auto_sparse_auto(
    source: &Path,
    dest: &Path,
) -> Result<(CopyDebug, CopyMethod), std::io::Error> {
    let mut copy_debug = CopyDebug {
        offload: OffloadReflinkDebug::Unknown,
        reflink: OffloadReflinkDebug::Unsupported,
        sparse_detection: SparseDebug::No,
    };

    let mut copy_method = CopyMethod::Default;
    let (data_flag, size, blocks) = check_for_data(source)?;
    let sparse_flag = check_sparse_detection(source)?;

    if (data_flag && size != 0) || (size > 0 && size < 512) {
        copy_debug.offload = OffloadReflinkDebug::Yes;
    }

    if data_flag && size == 0 {
        // Handling /proc/ files
        copy_debug.offload = OffloadReflinkDebug::Unsupported;
    }
    if sparse_flag {
        if blocks == 0 && data_flag {
            // Handling other "virtual" files
            copy_debug.offload = OffloadReflinkDebug::Unsupported;

            copy_method = CopyMethod::FSCopy; // Doing a standard copy for the virtual files
        } else {
            copy_method = CopyMethod::SparseCopyWithoutHole;
        } // Since sparse_flag is true, sparse_detection shall be SeekHole for any non virtual
          // regular sparse file and the file will be sparsely copied
        copy_debug.sparse_detection = SparseDebug::SeekHole;
    }

    if check_dest_is_fifo(dest) {
        copy_method = CopyMethod::FSCopy;
    }
    Ok((copy_debug, copy_method))
}

/// Handles debug results when flags are "--reflink=never" and "--sparse=auto" and specifies what
/// type of copy should be used
fn handle_reflink_never_sparse_auto(
    source: &Path,
    dest: &Path,
) -> Result<(CopyDebug, CopyMethod), std::io::Error> {
    let mut copy_debug = CopyDebug {
        offload: OffloadReflinkDebug::Unknown,
        reflink: OffloadReflinkDebug::No,
        sparse_detection: SparseDebug::No,
    };

    let (data_flag, size, blocks) = check_for_data(source)?;
    let sparse_flag = check_sparse_detection(source)?;

    let mut copy_method = CopyMethod::Default;
    if data_flag || size < 512 {
        copy_debug.offload = OffloadReflinkDebug::Avoided;
    }

    if sparse_flag {
        if blocks == 0 && data_flag {
            copy_method = CopyMethod::FSCopy; // Handles virtual files which have size > 0 but no
                                              // disk allocation
        } else {
            copy_method = CopyMethod::SparseCopyWithoutHole; // Handles regular sparse-files
        }
        copy_debug.sparse_detection = SparseDebug::SeekHole;
    }

    if check_dest_is_fifo(dest) {
        copy_method = CopyMethod::FSCopy;
    }
    Ok((copy_debug, copy_method))
}

/// Handles debug results when flags are "--reflink=never" and "--sparse=always" and specifies what
/// type of copy should be used
fn handle_reflink_never_sparse_always(
    source: &Path,
    dest: &Path,
) -> Result<(CopyDebug, CopyMethod), std::io::Error> {
    let mut copy_debug = CopyDebug {
        offload: OffloadReflinkDebug::Unknown,
        reflink: OffloadReflinkDebug::No,
        sparse_detection: SparseDebug::Zeros,
    };
    let mut copy_method = CopyMethod::SparseCopy;

    let (data_flag, size, blocks) = check_for_data(source)?;
    let sparse_flag = check_sparse_detection(source)?;

    if data_flag || size < 512 {
        copy_debug.offload = OffloadReflinkDebug::Avoided;
    }
    match (sparse_flag, data_flag, blocks) {
        (true, true, 0) => {
            // Handling funny files with 0 block allocation but has data
            // in it, e.g. files in /sys and other virtual files
            copy_method = CopyMethod::FSCopy;
            copy_debug.sparse_detection = SparseDebug::SeekHoleZeros;
        }
        (false, true, 0) => copy_method = CopyMethod::FSCopy, // Handling data containing zero sized
        // files in /proc
        (true, false, 0) => copy_debug.sparse_detection = SparseDebug::SeekHole, // Handles files
        // with 0 blocks allocated in disk and
        (true, true, _) => copy_debug.sparse_detection = SparseDebug::SeekHoleZeros, // Any
        // sparse_files with data in it will display SeekHoleZeros
        (true, false, _) => {
            copy_debug.offload = OffloadReflinkDebug::Unknown;
            copy_debug.sparse_detection = SparseDebug::SeekHole;
        }

        (_, _, _) => (),
    }
    if check_dest_is_fifo(dest) {
        copy_method = CopyMethod::FSCopy;
    }

    Ok((copy_debug, copy_method))
}
