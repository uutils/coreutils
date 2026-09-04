// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore reflink ftruncate fiemap lseek nofollow

use rustix::fs::{SeekFrom, ftruncate, ioctl_ficlone, seek};
use std::fs::File;
use std::io::{self, Read};
use std::os::unix::fs::FileExt;
use std::os::unix::fs::FileTypeExt;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

use uucore::buf_copy;
use uucore::display::Quotable;
use uucore::safe_copy::{create_dest_restrictive, open_source};
use uucore::translate;

use crate::{
    CopyDebug, CopyResult, CpError, OffloadReflinkDebug, ReflinkMode, SparseDebug, SparseMode,
    is_stream,
};

// Create the destination. It is followed when it is a pre-existing symlink,
// matching GNU cp -d/-P which only forbid dereferencing on the source side.
fn create_dest(dest: &Path) -> CopyResult<File> {
    create_dest_restrictive(dest, false).map_err(|e| {
        CpError::IoErrContext(
            e,
            translate!("cp-error-cannot-create-regular-file", "path" => dest.quote()),
        )
    })
}

/// The fallback behavior for [`clone`] on failed system call.
///
/// Every fallback reuses the descriptors already opened for the clone
/// attempt; re-opening the dest by path can fail with EACCES when the
/// umask stripped the write bits from its freshly created mode.
#[derive(Clone, Copy)]
enum CloneFallback {
    /// Raise an error.
    Error,

    /// Copy the bytes with [`buf_copy::copy_fast`].
    FSCopy,

    /// Use [`sparse_copy_fd`]
    SparseCopy,

    /// Use [`sparse_copy_without_hole_fd`]
    SparseCopyWithoutHole,
}

/// Type of method used for copying files
#[derive(Clone, Copy)]
enum CopyMethod {
    /// Do a sparse copy
    SparseCopy,
    /// Copy the bytes with [`buf_copy::copy_fast`].
    FSCopy,
    /// Default (can either be [`CopyMethod::SparseCopy`] or [`CopyMethod::FSCopy`])
    Default,
    /// Use [`sparse_copy_without_hole_fd`]
    SparseCopyWithoutHole,
}

/// Use the Linux `ioctl_ficlone` API to do a copy-on-write clone.
///
/// `fallback` controls what to do if the system call fails.
fn clone(
    src_file: &mut File,
    dest: &Path,
    fallback: CloneFallback,
    context: &str,
) -> CopyResult<()> {
    // Only needed to decide whether a failed --reflink=always clone should
    // clean up the dest, so skip the lstat for the other fallbacks.
    let dest_existed = matches!(fallback, CloneFallback::Error) && dest.symlink_metadata().is_ok();
    let mut dst_file = create_dest(dest)?;
    if let Err(err) = ioctl_ficlone(&dst_file, &*src_file) {
        // Reuse the already-open descriptors: the dest was just created with
        // a restrictive mode that the umask may have stripped of write bits,
        // so re-opening it by path can fail with EACCES (LP: #2164777).
        return match fallback {
            CloneFallback::Error => {
                // GNU cp removes a dest it created itself, but keeps a
                // pre-existing (now truncated) one. Only unlink while the
                // path still names the inode we created: it may have been
                // replaced since, and removing blindly would drop an
                // unrelated file.
                if !dest_existed && path_still_refers_to(dest, &dst_file) {
                    let _ = std::fs::remove_file(dest);
                }
                Err(CpError::IoErrContext(err.into(), context.to_owned()))
            }
            CloneFallback::FSCopy => buf_copy::copy_fast(src_file, &mut dst_file)
                .map_err(|e| CpError::IoErrContext(e, context.to_owned())),
            CloneFallback::SparseCopy => sparse_copy_fd(src_file, &dst_file, context),
            CloneFallback::SparseCopyWithoutHole => {
                sparse_copy_without_hole_fd(src_file, &dst_file, context)
            }
        };
    }
    Ok(())
}

/// Whether `path` still resolves to the inode behind `file`.
fn path_still_refers_to(path: &Path, file: &File) -> bool {
    let (Ok(current), Ok(opened)) = (path.symlink_metadata(), file.metadata()) else {
        return false;
    };
    current.dev() == opened.dev() && current.ino() == opened.ino()
}

/// Checks whether a file contains any non null bytes i.e. any byte != 0x0
/// This function returns a tuple of (bool, u64, u64) signifying a tuple of (whether a file has
/// data, its size, no of blocks it has allocated in disk)
fn check_for_data(src_file: &mut File) -> io::Result<(bool, u64, u64)> {
    let metadata = src_file.metadata()?;

    let size = metadata.size();
    let blocks = metadata.blocks();
    // checks edge case of virtual files in /proc which have a size of zero but contains data
    let (has_data, blocks) = if size == 0 {
        let mut buf: Vec<u8> = vec![0; metadata.blksize() as usize]; // Directly use metadata.blksize()
        let read = src_file.read(&mut buf)?;
        (buf[..read].iter().any(|&x| x != 0x0), 0)
    } else {
        (seek(&*src_file, SeekFrom::Data(0)).is_ok(), blocks)
    };

    // The probe moved the descriptor; the copy that follows reads it
    // sequentially, so hand it back positioned at the start.
    seek(&*src_file, SeekFrom::Start(0))?;

    Ok((has_data, size, blocks))
}

/// Checks whether a file is sparse i.e. it contains holes, uses the crude heuristic blocks < size / 512
/// Reference:`<https://doc.rust-lang.org/std/os/unix/fs/trait.MetadataExt.html#tymethod.blocks>`
fn check_sparse_detection(src_file: &File) -> io::Result<bool> {
    let metadata = src_file.metadata()?;
    let size = metadata.size();
    let blocks = metadata.blocks();

    Ok(blocks < size / 512)
}

/// Optimized [`sparse_copy_fd`] doesn't create holes for large sequences of zeros in non `sparse_files`
/// Used when `--sparse=auto`
fn sparse_copy_without_hole_fd(src_file: &File, dst_file: &File, context: &str) -> CopyResult<()> {
    let ctx_err = |e: io::Error| CpError::IoErrContext(e, context.to_owned());

    let size = src_file.metadata().map_err(&ctx_err)?.size();
    ftruncate(dst_file, size).map_err(|e| CpError::IoErrContext(e.into(), context.to_owned()))?;
    let mut current_offset = 0;
    // Maximize the data read at once to 16 MiB to avoid memory hogging with large files
    // 16 MiB chunks should saturate an SSD
    // At least 1 byte, so that a source that was empty at fstat time but
    // gained data before the SEEK_DATA loop cannot make `step_by` panic.
    let step = size.clamp(1, 16 * 1024 * 1024) as usize;
    let mut buf: Vec<u8> = vec![0x0; step];
    while let Ok(data) = seek(src_file, SeekFrom::Data(current_offset)) {
        current_offset = data;
        let Ok(hole) = seek(src_file, SeekFrom::Hole(current_offset)) else {
            break;
        };
        let len = hole - current_offset;
        // Read and write data in chunks of `step` while reusing the same buffer
        for i in (0..len).step_by(step) {
            // Ensure we don't read past the end of the file or the start of
            // the next hole. Take the min in u64: casting `len - i` first
            // would truncate extents of 4 GiB and more on 32-bit targets.
            let read_len = std::cmp::min(len - i, step as u64) as usize;
            let buf = &mut buf[..read_len];
            src_file
                .read_exact_at(buf, current_offset + i)
                .map_err(&ctx_err)?;
            dst_file
                .write_all_at(buf, current_offset + i)
                .map_err(&ctx_err)?;
        }
        current_offset = hole;
    }
    Ok(())
}
/// Perform a sparse copy from one file to another.
/// Creates a holes for large sequences of zeros in `non_sparse_files`, used for `--sparse=always`
fn sparse_copy_fd(src_file: &mut File, dst_file: &File, context: &str) -> CopyResult<()> {
    let ctx_err = |e: io::Error| CpError::IoErrContext(e, context.to_owned());

    // Keep the size as u64: on 32-bit targets a usize conversion would
    // panic for sources of 4 GiB and more.
    let size = src_file.metadata().map_err(&ctx_err)?.size();
    ftruncate(dst_file, size).map_err(|e| CpError::IoErrContext(e.into(), context.to_owned()))?;

    let blksize = dst_file.metadata().map_err(&ctx_err)?.blksize();
    let mut buf: Vec<u8> = vec![0; blksize as usize];
    let mut current_offset: u64 = 0;

    // TODO Perhaps we can employ the "fiemap ioctl" API to get the
    // file extent mappings:
    // https://www.kernel.org/doc/html/latest/filesystems/fiemap.html
    while current_offset < size {
        let this_read = src_file.read(&mut buf).map_err(&ctx_err)?;
        if this_read == 0 {
            // EOF before the size seen at fstat time (source truncated
            // concurrently): shrink the dest to the bytes actually copied
            // instead of leaving a zero-filled tail up to the stale size.
            ftruncate(dst_file, current_offset)
                .map_err(|e| CpError::IoErrContext(e.into(), context.to_owned()))?;
            break;
        }
        let buf = &buf[..this_read];
        if buf.iter().any(|&x| x != 0) {
            dst_file
                .write_all_at(buf, current_offset)
                .map_err(&ctx_err)?;
        }
        current_offset += this_read as u64;
    }
    Ok(())
}

/// Checks whether an existing destination is a fifo
fn check_dest_is_fifo(dest: &Path) -> bool {
    // If our destination file exists and its a fifo , we do a standard copy .
    std::fs::metadata(dest).is_ok_and(|f| f.file_type().is_fifo())
}

/// Copy the contents of a stream from `source` to `dest`.
fn copy_stream<P>(source: P, dest: P, nofollow: bool, context: &str) -> CopyResult<()>
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
    let mut src_file =
        open_source(&source, nofollow).map_err(|e| CpError::IoErrContext(e, context.to_owned()))?;
    // Use the same restrictive initial mode as the regular file path so that
    // the dest does not momentarily sit with broader perms. The `0o622 &
    // !umask` form previously used here could still allow group/other write
    // under a permissive umask. See #10011.
    let mut dst_file = create_dest_restrictive(&dest, false).map_err(|e| {
        CpError::IoErrContext(
            e,
            translate!("cp-error-cannot-create-regular-file", "path" => dest.as_ref().quote()),
        )
    })?;

    let ctx_err = |e: io::Error| CpError::IoErrContext(e, context.to_owned());

    let dest_is_stream = is_stream(&dst_file.metadata().map_err(&ctx_err)?);
    if !dest_is_stream {
        // `copy_stream` doesn't clear the dest file, if dest is not a stream, we should clear it manually.
        dst_file.set_len(0).map_err(&ctx_err)?;
    }

    buf_copy::copy_fast(&mut src_file, &mut dst_file)
        .map_err(|e| io::Error::other(format!("{e}")))
        .map_err(&ctx_err)?;

    Ok(())
}

/// Copies `source` to `dest` using copy-on-write if possible.
///
/// The source is opened once and the descriptor is threaded through both the
/// sparseness probe and the copy itself. Probing and copying used to open the
/// path separately, costing three opens per copy and letting the strategy be
/// decided from one file while the bytes came from whatever the path named
/// later (#13185).
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
                copy_stream(source, dest, nofollow, context)
            } else {
                let mut src_file = open_source(source, nofollow)
                    .map_err(|e| CpError::IoErrContext(e, context.to_owned()))?;
                let mut copy_method = CopyMethod::Default;
                let result = handle_reflink_never_sparse_always(&mut src_file, dest);
                if let Ok((debug, method)) = result {
                    copy_debug = debug;
                    copy_method = method;
                }

                let mut dst_file = create_dest(dest)?;
                match copy_method {
                    CopyMethod::FSCopy => buf_copy::copy_fast(&mut src_file, &mut dst_file)
                        .map_err(|e| CpError::IoErrContext(e, context.to_owned())),
                    _ => sparse_copy_fd(&mut src_file, &dst_file, context),
                }
            }
        }
        (ReflinkMode::Never, SparseMode::Never) => {
            copy_debug.reflink = OffloadReflinkDebug::No;

            if source_is_stream {
                copy_debug.offload = OffloadReflinkDebug::Avoided;
                copy_stream(source, dest, nofollow, context)
            } else {
                let mut src_file = open_source(source, nofollow)
                    .map_err(|e| CpError::IoErrContext(e, context.to_owned()))?;
                let result = handle_reflink_never_sparse_never(&mut src_file);
                if let Ok(debug) = result {
                    copy_debug = debug;
                }
                let mut dst_file = create_dest(dest)?;
                buf_copy::copy_fast(&mut src_file, &mut dst_file)
                    .map_err(|e| CpError::IoErrContext(e, context.to_owned()))
            }
        }
        (ReflinkMode::Never, SparseMode::Auto) => {
            copy_debug.reflink = OffloadReflinkDebug::No;

            if source_is_stream {
                copy_debug.offload = OffloadReflinkDebug::Avoided;
                copy_stream(source, dest, nofollow, context)
            } else {
                let mut src_file = open_source(source, nofollow)
                    .map_err(|e| CpError::IoErrContext(e, context.to_owned()))?;
                let mut copy_method = CopyMethod::Default;
                let result = handle_reflink_never_sparse_auto(&mut src_file, dest);
                if let Ok((debug, method)) = result {
                    copy_debug = debug;
                    copy_method = method;
                }

                let mut dst_file = create_dest(dest)?;
                match copy_method {
                    CopyMethod::SparseCopyWithoutHole => {
                        sparse_copy_without_hole_fd(&src_file, &dst_file, context)
                    }
                    _ => buf_copy::copy_fast(&mut src_file, &mut dst_file)
                        .map_err(|e| CpError::IoErrContext(e, context.to_owned())),
                }
            }
        }
        (ReflinkMode::Auto, SparseMode::Always) => {
            copy_debug.sparse_detection = SparseDebug::Zeros; // Default SparseDebug val for
            // SparseMode::Always
            if source_is_stream {
                copy_debug.offload = OffloadReflinkDebug::Avoided;
                copy_stream(source, dest, nofollow, context)
            } else {
                let mut src_file = open_source(source, nofollow)
                    .map_err(|e| CpError::IoErrContext(e, context.to_owned()))?;
                let mut copy_method = CopyMethod::Default;
                let result = handle_reflink_auto_sparse_always(&mut src_file, dest);
                if let Ok((debug, method)) = result {
                    copy_debug = debug;
                    copy_method = method;
                }

                match copy_method {
                    CopyMethod::FSCopy => {
                        clone(&mut src_file, dest, CloneFallback::FSCopy, context)
                    }
                    _ => clone(&mut src_file, dest, CloneFallback::SparseCopy, context),
                }
            }
        }

        (ReflinkMode::Auto, SparseMode::Never) => {
            copy_debug.reflink = OffloadReflinkDebug::No;
            if source_is_stream {
                copy_debug.offload = OffloadReflinkDebug::Avoided;
                copy_stream(source, dest, nofollow, context)
            } else {
                let mut src_file = open_source(source, nofollow)
                    .map_err(|e| CpError::IoErrContext(e, context.to_owned()))?;
                let result = handle_reflink_auto_sparse_never(&mut src_file);
                if let Ok(debug) = result {
                    copy_debug = debug;
                }

                clone(&mut src_file, dest, CloneFallback::FSCopy, context)
            }
        }
        (ReflinkMode::Auto, SparseMode::Auto) => {
            if source_is_stream {
                copy_debug.offload = OffloadReflinkDebug::Unsupported;
                copy_stream(source, dest, nofollow, context)
            } else {
                let mut src_file = open_source(source, nofollow)
                    .map_err(|e| CpError::IoErrContext(e, context.to_owned()))?;
                let mut copy_method = CopyMethod::Default;
                let result = handle_reflink_auto_sparse_auto(&mut src_file, dest);
                if let Ok((debug, method)) = result {
                    copy_debug = debug;
                    copy_method = method;
                }

                match copy_method {
                    CopyMethod::SparseCopyWithoutHole => clone(
                        &mut src_file,
                        dest,
                        CloneFallback::SparseCopyWithoutHole,
                        context,
                    ),
                    _ => clone(&mut src_file, dest, CloneFallback::FSCopy, context),
                }
            }
        }

        (ReflinkMode::Always, SparseMode::Auto) => {
            copy_debug.sparse_detection = SparseDebug::No;
            copy_debug.reflink = OffloadReflinkDebug::Yes;

            let mut src_file = open_source(source, nofollow)
                .map_err(|e| CpError::IoErrContext(e, context.to_owned()))?;
            clone(&mut src_file, dest, CloneFallback::Error, context)
        }
        (ReflinkMode::Always, _) => {
            return Err(translate!("cp-error-reflink-always-sparse-auto").into());
        }
    };
    result?;
    Ok(copy_debug)
}

/// Handles debug results when flags are "--reflink=auto" and "--sparse=always" and specifies what
/// type of copy should be used
fn handle_reflink_auto_sparse_always(
    src_file: &mut File,
    dest: &Path,
) -> io::Result<(CopyDebug, CopyMethod)> {
    let mut copy_debug = CopyDebug {
        offload: OffloadReflinkDebug::Unknown,
        reflink: OffloadReflinkDebug::Unsupported,
        sparse_detection: SparseDebug::Zeros,
    };
    let mut copy_method = CopyMethod::Default;
    let (data_flag, size, blocks) = check_for_data(src_file)?;
    let sparse_flag = check_sparse_detection(src_file)?;

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
fn handle_reflink_never_sparse_never(src_file: &mut File) -> io::Result<CopyDebug> {
    let mut copy_debug = CopyDebug {
        offload: OffloadReflinkDebug::Unknown,
        reflink: OffloadReflinkDebug::No,
        sparse_detection: SparseDebug::No,
    };
    let (data_flag, size, _blocks) = check_for_data(src_file)?;
    let sparse_flag = check_sparse_detection(src_file)?;

    if sparse_flag {
        copy_debug.sparse_detection = SparseDebug::SeekHole;
    }

    if data_flag || size < 512 {
        copy_debug.offload = OffloadReflinkDebug::Avoided;
    }
    Ok(copy_debug)
}

/// Handles debug results when flags are "--reflink=auto" and "--sparse=never", files will be copied
/// through cloning them with fallback switching to [`buf_copy::copy_fast`]
fn handle_reflink_auto_sparse_never(src_file: &mut File) -> io::Result<CopyDebug> {
    let mut copy_debug = CopyDebug {
        offload: OffloadReflinkDebug::Unknown,
        reflink: OffloadReflinkDebug::No,
        sparse_detection: SparseDebug::No,
    };

    let (data_flag, size, _blocks) = check_for_data(src_file)?;
    let sparse_flag = check_sparse_detection(src_file)?;

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
    src_file: &mut File,
    dest: &Path,
) -> io::Result<(CopyDebug, CopyMethod)> {
    let mut copy_debug = CopyDebug {
        offload: OffloadReflinkDebug::Unknown,
        reflink: OffloadReflinkDebug::Unsupported,
        sparse_detection: SparseDebug::No,
    };

    let mut copy_method = CopyMethod::Default;
    let (data_flag, size, blocks) = check_for_data(src_file)?;
    let sparse_flag = check_sparse_detection(src_file)?;

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
    src_file: &mut File,
    dest: &Path,
) -> io::Result<(CopyDebug, CopyMethod)> {
    let mut copy_debug = CopyDebug {
        offload: OffloadReflinkDebug::Unknown,
        reflink: OffloadReflinkDebug::No,
        sparse_detection: SparseDebug::No,
    };

    let (data_flag, size, blocks) = check_for_data(src_file)?;
    let sparse_flag = check_sparse_detection(src_file)?;

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
    src_file: &mut File,
    dest: &Path,
) -> io::Result<(CopyDebug, CopyMethod)> {
    let mut copy_debug = CopyDebug {
        offload: OffloadReflinkDebug::Unknown,
        reflink: OffloadReflinkDebug::No,
        sparse_detection: SparseDebug::Zeros,
    };
    let mut copy_method = CopyMethod::SparseCopy;

    let (data_flag, size, blocks) = check_for_data(src_file)?;
    let sparse_flag = check_sparse_detection(src_file)?;

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
