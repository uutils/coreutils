// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore ficlone reflink ftruncate pwrite fiemap lseek nofollow

use rustix::fs::{SeekFrom, ftruncate, ioctl_ficlone, seek};
use std::io::Read;
use std::os::unix::fs::FileExt;
use std::os::unix::fs::FileTypeExt;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

use uucore::buf_copy;
use uucore::safe_copy::{create_dest_restrictive, open_source};
use uucore::translate;

use crate::{
    CopyDebug, CopyResult, CpError, OffloadReflinkDebug, ReflinkMode, SparseDebug, SparseMode,
    is_stream,
};

// Replacement for `std::fs::copy` that uses the safe-copy primitives but
// only applies `O_NOFOLLOW` to the *source* open. The destination is
// followed if it is a pre-existing symlink, matching GNU cp -d/-P which
// only forbid dereferencing on the source side.
fn fs_copy<P, Q>(source: P, dest: Q, source_nofollow: bool) -> std::io::Result<u64>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let mut src = open_source(source, source_nofollow)?;
    let mut dst = create_dest_restrictive(dest, false)?;
    std::io::copy(&mut src, &mut dst)
}

/// The fallback behavior for [`clone`] on failed system call.
#[derive(Clone, Copy)]
enum CloneFallback {
    /// Raise an error.
    Error,

    /// Use [`std::fs::copy`].
    FSCopy,

    /// Use [`sparse_copy`]
    SparseCopy,

    /// Use [`sparse_copy_without_hole`]
    SparseCopyWithoutHole,
}

/// Type of method used for copying files
#[derive(Clone, Copy)]
enum CopyMethod {
    /// Do a sparse copy
    SparseCopy,
    /// Use [`std::fs::copy`].
    FSCopy,
    /// Default (can either be [`CopyMethod::SparseCopy`] or [`CopyMethod::FSCopy`])
    Default,
    /// Use [`sparse_copy_without_hole`]
    SparseCopyWithoutHole,
}

/// Use the Linux `ioctl_ficlone` API to do a copy-on-write clone.
///
/// `fallback` controls what to do if the system call fails.
#[cfg(any(target_os = "linux", target_os = "android"))]
fn clone<P>(source: P, dest: P, fallback: CloneFallback, nofollow: bool) -> std::io::Result<()>
where
    P: AsRef<Path>,
{
    let src_file = open_source(&source, nofollow)?;
    let dst_file = create_dest_restrictive(&dest, false)?;
    if ioctl_ficlone(dst_file, src_file).is_err() {
        return match fallback {
            CloneFallback::Error => Err(std::io::Error::last_os_error()),
            CloneFallback::FSCopy => fs_copy(source, dest, nofollow).map(|_| ()),
            CloneFallback::SparseCopy => sparse_copy(source, dest, nofollow),
            CloneFallback::SparseCopyWithoutHole => {
                sparse_copy_without_hole(source, dest, nofollow)
            }
        };
    }
    Ok(())
}

/// Checks whether a file contains any non null bytes i.e. any byte != 0x0
/// This function returns a tuple of (bool, u64, u64) signifying a tuple of (whether a file has
/// data, its size, no of blocks it has allocated in disk)
#[cfg(any(target_os = "linux", target_os = "android"))]
fn check_for_data(source: &Path, nofollow: bool) -> Result<(bool, u64, u64), std::io::Error> {
    let mut src_file = open_source(source, nofollow)?;
    let metadata = src_file.metadata()?;

    let size = metadata.size();
    let blocks = metadata.blocks();
    // checks edge case of virtual files in /proc which have a size of zero but contains data
    if size == 0 {
        let mut buf: Vec<u8> = vec![0; metadata.blksize() as usize]; // Directly use metadata.blksize()
        let _ = src_file.read(&mut buf)?;
        return Ok((buf.iter().any(|&x| x != 0x0), size, 0));
    }
    let has_data = seek(src_file, SeekFrom::Data(0)).is_ok();

    Ok((has_data, size, blocks))
}

#[cfg(any(target_os = "linux", target_os = "android"))]
/// Checks whether a file is sparse i.e. it contains holes, uses the crude heuristic blocks < size / 512
/// Reference:`<https://doc.rust-lang.org/std/os/unix/fs/trait.MetadataExt.html#tymethod.blocks>`
fn check_sparse_detection(source: &Path, nofollow: bool) -> Result<bool, std::io::Error> {
    let src_file = open_source(source, nofollow)?;
    let metadata = src_file.metadata()?;
    let size = metadata.size();
    let blocks = metadata.blocks();

    if blocks < size / 512 {
        return Ok(true);
    }
    Ok(false)
}

/// Optimized [`sparse_copy`] doesn't create holes for large sequences of zeros in non `sparse_files`
/// Used when `--sparse=auto`
#[cfg(any(target_os = "linux", target_os = "android"))]
fn sparse_copy_without_hole<P>(source: P, dest: P, nofollow: bool) -> std::io::Result<()>
where
    P: AsRef<Path>,
{
    let src_file = open_source(source, nofollow)?;
    let dst_file = create_dest_restrictive(dest, false)?;

    let size = src_file.metadata()?.size();
    ftruncate(&dst_file, size)?;
    let mut current_offset = 0;
    // Maximize the data read at once to 16 MiB to avoid memory hogging with large files
    // 16 MiB chunks should saturate an SSD
    let step = std::cmp::min(size, 16 * 1024 * 1024) as usize;
    let mut buf: Vec<u8> = vec![0x0; step];
    while let Ok(data) = seek(&src_file, SeekFrom::Data(current_offset)) {
        current_offset = data;
        let Ok(hole) = seek(&src_file, SeekFrom::Hole(current_offset)) else {
            break;
        };
        let len = hole - current_offset;
        // Read and write data in chunks of `step` while reusing the same buffer
        for i in (0..len).step_by(step) {
            // Ensure we don't read past the end of the file or the start of the next hole
            let read_len = std::cmp::min((len - i) as usize, step);
            let buf = &mut buf[..read_len];
            src_file.read_exact_at(buf, current_offset + i)?;
            dst_file.write_all_at(buf, current_offset + i)?;
        }
        current_offset = hole;
    }
    Ok(())
}
/// Perform a sparse copy from one file to another.
/// Creates a holes for large sequences of zeros in `non_sparse_files`, used for `--sparse=always`
#[cfg(any(target_os = "linux", target_os = "android"))]
fn sparse_copy<P>(source: P, dest: P, nofollow: bool) -> std::io::Result<()>
where
    P: AsRef<Path>,
{
    let mut src_file = open_source(source, nofollow)?;
    let dst_file = create_dest_restrictive(dest, false)?;

    let size: usize = src_file.metadata()?.size().try_into().unwrap();
    ftruncate(&dst_file, size.try_into().unwrap())?;

    let blksize = dst_file.metadata()?.blksize();
    let mut buf: Vec<u8> = vec![0; blksize.try_into().unwrap()];
    let mut current_offset: usize = 0;

    // TODO Perhaps we can employ the "fiemap ioctl" API to get the
    // file extent mappings:
    // https://www.kernel.org/doc/html/latest/filesystems/fiemap.html
    while current_offset < size {
        let this_read = src_file.read(&mut buf)?;
        let buf = &buf[..this_read];
        if buf.iter().any(|&x| x != 0) {
            dst_file.write_all_at(buf, current_offset.try_into().unwrap())?;
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

/// Copy the contents of a stream from `source` to `dest`.
fn copy_stream<P>(source: P, dest: P, nofollow: bool) -> std::io::Result<()>
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
    let mut src_file = open_source(&source, nofollow)?;
    // Use the same restrictive initial mode as the regular file path so that
    // the dest does not momentarily sit with broader perms. The `0o622 &
    // !umask` form previously used here could still allow group/other write
    // under a permissive umask. See #10011.
    let mut dst_file = create_dest_restrictive(&dest, false)?;

    let dest_is_stream = is_stream(&dst_file.metadata()?);
    if !dest_is_stream {
        // `copy_stream` doesn't clear the dest file, if dest is not a stream, we should clear it manually.
        dst_file.set_len(0)?;
    }

    buf_copy::copy_stream(&mut src_file, &mut dst_file)
        .map_err(|e| std::io::Error::other(format!("{e}")))?;

    Ok(())
}

/// Copies `source` to `dest` using copy-on-write if possible.
pub(crate) fn copy_on_write(
    source: &Path,
    dest: &Path,
    reflink_mode: ReflinkMode,
    sparse_mode: SparseMode,
    context: &str,
    source_is_stream: bool,
    nofollow: bool,
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
            if source_is_stream {
                copy_debug.offload = OffloadReflinkDebug::Avoided;
                copy_stream(source, dest, nofollow).map(|_| ())
            } else {
                let mut copy_method = CopyMethod::Default;
                let result = handle_reflink_never_sparse_always(source, dest, nofollow);
                if let Ok((debug, method)) = result {
                    copy_debug = debug;
                    copy_method = method;
                }

                match copy_method {
                    CopyMethod::FSCopy => fs_copy(source, dest, nofollow).map(|_| ()),
                    _ => sparse_copy(source, dest, nofollow),
                }
            }
        }
        (ReflinkMode::Never, SparseMode::Never) => {
            copy_debug.reflink = OffloadReflinkDebug::No;

            if source_is_stream {
                copy_debug.offload = OffloadReflinkDebug::Avoided;
                copy_stream(source, dest, nofollow).map(|_| ())
            } else {
                let result = handle_reflink_never_sparse_never(source, nofollow);
                if let Ok(debug) = result {
                    copy_debug = debug;
                }
                fs_copy(source, dest, nofollow).map(|_| ())
            }
        }
        (ReflinkMode::Never, SparseMode::Auto) => {
            copy_debug.reflink = OffloadReflinkDebug::No;

            if source_is_stream {
                copy_debug.offload = OffloadReflinkDebug::Avoided;
                copy_stream(source, dest, nofollow).map(|_| ())
            } else {
                let mut copy_method = CopyMethod::Default;
                let result = handle_reflink_never_sparse_auto(source, dest, nofollow);
                if let Ok((debug, method)) = result {
                    copy_debug = debug;
                    copy_method = method;
                }

                match copy_method {
                    CopyMethod::SparseCopyWithoutHole => {
                        sparse_copy_without_hole(source, dest, nofollow)
                    }
                    _ => fs_copy(source, dest, nofollow).map(|_| ()),
                }
            }
        }
        (ReflinkMode::Auto, SparseMode::Always) => {
            copy_debug.sparse_detection = SparseDebug::Zeros; // Default SparseDebug val for
            // SparseMode::Always
            if source_is_stream {
                copy_debug.offload = OffloadReflinkDebug::Avoided;
                copy_stream(source, dest, nofollow).map(|_| ())
            } else {
                let mut copy_method = CopyMethod::Default;
                let result = handle_reflink_auto_sparse_always(source, dest, nofollow);
                if let Ok((debug, method)) = result {
                    copy_debug = debug;
                    copy_method = method;
                }

                match copy_method {
                    CopyMethod::FSCopy => clone(source, dest, CloneFallback::FSCopy, nofollow),
                    _ => clone(source, dest, CloneFallback::SparseCopy, nofollow),
                }
            }
        }

        (ReflinkMode::Auto, SparseMode::Never) => {
            copy_debug.reflink = OffloadReflinkDebug::No;
            if source_is_stream {
                copy_debug.offload = OffloadReflinkDebug::Avoided;
                copy_stream(source, dest, nofollow).map(|_| ())
            } else {
                let result = handle_reflink_auto_sparse_never(source, nofollow);
                if let Ok(debug) = result {
                    copy_debug = debug;
                }

                clone(source, dest, CloneFallback::FSCopy, nofollow)
            }
        }
        (ReflinkMode::Auto, SparseMode::Auto) => {
            if source_is_stream {
                copy_debug.offload = OffloadReflinkDebug::Unsupported;
                copy_stream(source, dest, nofollow).map(|_| ())
            } else {
                let mut copy_method = CopyMethod::Default;
                let result = handle_reflink_auto_sparse_auto(source, dest, nofollow);
                if let Ok((debug, method)) = result {
                    copy_debug = debug;
                    copy_method = method;
                }

                match copy_method {
                    CopyMethod::SparseCopyWithoutHole => {
                        clone(source, dest, CloneFallback::SparseCopyWithoutHole, nofollow)
                    }
                    _ => clone(source, dest, CloneFallback::FSCopy, nofollow),
                }
            }
        }

        (ReflinkMode::Always, SparseMode::Auto) => {
            copy_debug.sparse_detection = SparseDebug::No;
            copy_debug.reflink = OffloadReflinkDebug::Yes;

            clone(source, dest, CloneFallback::Error, nofollow)
        }
        (ReflinkMode::Always, _) => {
            return Err(translate!("cp-error-reflink-always-sparse-auto").into());
        }
    };
    result.map_err(|e| CpError::IoErrContext(e, context.to_owned()))?;
    Ok(copy_debug)
}

/// Handles debug results when flags are "--reflink=auto" and "--sparse=always" and specifies what
/// type of copy should be used
fn handle_reflink_auto_sparse_always(
    source: &Path,
    dest: &Path,
    nofollow: bool,
) -> Result<(CopyDebug, CopyMethod), std::io::Error> {
    let mut copy_debug = CopyDebug {
        offload: OffloadReflinkDebug::Unknown,
        reflink: OffloadReflinkDebug::Unsupported,
        sparse_detection: SparseDebug::Zeros,
    };
    let mut copy_method = CopyMethod::Default;
    let (data_flag, size, blocks) = check_for_data(source, nofollow)?;
    let sparse_flag = check_sparse_detection(source, nofollow)?;

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
fn handle_reflink_never_sparse_never(
    source: &Path,
    nofollow: bool,
) -> Result<CopyDebug, std::io::Error> {
    let mut copy_debug = CopyDebug {
        offload: OffloadReflinkDebug::Unknown,
        reflink: OffloadReflinkDebug::No,
        sparse_detection: SparseDebug::No,
    };
    let (data_flag, size, _blocks) = check_for_data(source, nofollow)?;
    let sparse_flag = check_sparse_detection(source, nofollow)?;

    if sparse_flag {
        copy_debug.sparse_detection = SparseDebug::SeekHole;
    }

    if data_flag || size < 512 {
        copy_debug.offload = OffloadReflinkDebug::Avoided;
    }
    Ok(copy_debug)
}

/// Handles debug results when flags are "--reflink=auto" and "--sparse=never", files will be copied
/// through cloning them with fallback switching to [`std::fs::copy`]
fn handle_reflink_auto_sparse_never(
    source: &Path,
    nofollow: bool,
) -> Result<CopyDebug, std::io::Error> {
    let mut copy_debug = CopyDebug {
        offload: OffloadReflinkDebug::Unknown,
        reflink: OffloadReflinkDebug::No,
        sparse_detection: SparseDebug::No,
    };

    let (data_flag, size, _blocks) = check_for_data(source, nofollow)?;
    let sparse_flag = check_sparse_detection(source, nofollow)?;

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
    nofollow: bool,
) -> Result<(CopyDebug, CopyMethod), std::io::Error> {
    let mut copy_debug = CopyDebug {
        offload: OffloadReflinkDebug::Unknown,
        reflink: OffloadReflinkDebug::Unsupported,
        sparse_detection: SparseDebug::No,
    };

    let mut copy_method = CopyMethod::Default;
    let (data_flag, size, blocks) = check_for_data(source, nofollow)?;
    let sparse_flag = check_sparse_detection(source, nofollow)?;

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
    nofollow: bool,
) -> Result<(CopyDebug, CopyMethod), std::io::Error> {
    let mut copy_debug = CopyDebug {
        offload: OffloadReflinkDebug::Unknown,
        reflink: OffloadReflinkDebug::No,
        sparse_detection: SparseDebug::No,
    };

    let (data_flag, size, blocks) = check_for_data(source, nofollow)?;
    let sparse_flag = check_sparse_detection(source, nofollow)?;

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
    nofollow: bool,
) -> Result<(CopyDebug, CopyMethod), std::io::Error> {
    let mut copy_debug = CopyDebug {
        offload: OffloadReflinkDebug::Unknown,
        reflink: OffloadReflinkDebug::No,
        sparse_detection: SparseDebug::Zeros,
    };
    let mut copy_method = CopyMethod::SparseCopy;

    let (data_flag, size, blocks) = check_for_data(source, nofollow)?;
    let sparse_flag = check_sparse_detection(source, nofollow)?;

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
