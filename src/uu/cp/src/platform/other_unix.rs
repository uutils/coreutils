// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore reflink
use std::fs::{self, File, OpenOptions};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

use uucore::buf_copy;
use uucore::mode::get_umask;
use uucore::translate;

use crate::{
    CopyDebug, CopyResult, CpError, OffloadReflinkDebug, ReflinkMode, SparseDebug, SparseMode,
    is_stream,
};

/// Open a source file for reading, optionally preventing symlink following.
fn open_source(path: &Path, follow_symlinks: bool) -> std::io::Result<File> {
    let mut opts = OpenOptions::new();
    opts.read(true);
    if !follow_symlinks {
        opts.custom_flags(libc::O_NOFOLLOW);
    }
    opts.open(path)
}

/// Copies `source` to `dest` for systems without copy-on-write
pub(crate) fn copy_on_write(
    source: &Path,
    dest: &Path,
    reflink_mode: ReflinkMode,
    sparse_mode: SparseMode,
    context: &str,
    source_is_stream: bool,
    follow_symlinks: bool,
) -> CopyResult<CopyDebug> {
    if reflink_mode != ReflinkMode::Never {
        return Err(translate!("cp-error-reflink-not-supported")
            .to_string()
            .into());
    }
    if sparse_mode != SparseMode::Auto {
        return Err(translate!("cp-error-sparse-not-supported")
            .to_string()
            .into());
    }
    let copy_debug = CopyDebug {
        offload: OffloadReflinkDebug::Unsupported,
        reflink: OffloadReflinkDebug::Unsupported,
        sparse_detection: SparseDebug::Unsupported,
    };

    if source_is_stream {
        let mut src_file = open_source(source, follow_symlinks)?;
        let mode = 0o622 & !get_umask();
        let mut dst_file = OpenOptions::new()
            .create(true)
            .write(true)
            .mode(mode)
            .open(dest)?;

        let dest_is_stream = is_stream(&dst_file.metadata()?);
        if !dest_is_stream {
            // `copy_stream` doesn't clear the dest file, if dest is not a stream, we should clear it manually.
            dst_file.set_len(0)?;
        }

        buf_copy::copy_stream(&mut src_file, &mut dst_file)
            .map_err(|_| std::io::Error::from(std::io::ErrorKind::Other))
            .map_err(|e| CpError::IoErrContext(e, context.to_owned()))?;

        return Ok(copy_debug);
    }

    let mut src_file = open_source(source, follow_symlinks)
        .map_err(|e| CpError::IoErrContext(e, context.to_owned()))?;
    let mut dst_file =
        File::create(dest).map_err(|e| CpError::IoErrContext(e, context.to_owned()))?;
    buf_copy::copy_stream(&mut src_file, &mut dst_file)
        .map_err(|e| std::io::Error::other(format!("{e}")))
        .map_err(|e| CpError::IoErrContext(e, context.to_owned()))?;

    Ok(copy_debug)
}
