// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore reflink
use std::fs::File;
use std::path::Path;

use uucore::buf_copy;
use uucore::safe_copy::create_dest_restrictive;
use uucore::translate;

use crate::{
    CopyDebug, CopyResult, CpError, OffloadReflinkDebug, ReflinkMode, SparseDebug, SparseMode,
    is_stream,
};

/// Copies `source` to `dest` for systems without copy-on-write
pub(crate) fn copy_on_write(
    source: &Path,
    dest: &Path,
    reflink_mode: ReflinkMode,
    sparse_mode: SparseMode,
    context: &str,
    source_is_stream: bool,
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
        let mut src_file = File::open(source)?;
        let mut dst_file = create_dest_restrictive(dest, false)?;

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

    // Equivalent of fs::copy but creates dest with DEST_INITIAL_MODE rather
    // than the default umask-derived 0o666, closing the window where another
    // user could read/write dest before cp applies the final permissions.
    let mut src_file =
        File::open(source).map_err(|e| CpError::IoErrContext(e, context.to_owned()))?;
    let mut dst_file =
        create_dest_restrictive(dest, false).map_err(|e| CpError::IoErrContext(e, context.to_owned()))?;
    std::io::copy(&mut src_file, &mut dst_file)
        .map_err(|e| CpError::IoErrContext(e, context.to_owned()))?;

    Ok(copy_debug)
}
