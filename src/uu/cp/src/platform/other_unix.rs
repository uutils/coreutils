// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore reflink
use std::fs::{File, OpenOptions};
use std::io;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

use uucore::buf_copy;
use uucore::fs::copy_file_with_secure_permissions;
use uucore::translate;

use crate::{
    CopyDebug, CopyResult, CpError, OffloadReflinkDebug, ReflinkMode, SparseDebug, SparseMode,
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
        // Create with restrictive permissions initially to prevent race conditions
        // Mode 0o600 means read/write for owner only
        let mut dst_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .mode(0o600)
            .open(dest)?;

        buf_copy::copy_stream(&mut src_file, &mut dst_file)
            .map_err(|_| io::Error::from(io::ErrorKind::Other))
            .map_err(|e| CpError::IoErrContext(e, context.to_owned()))?;

        return Ok(copy_debug);
    }

    copy_file_with_secure_permissions(source, dest)
        .map_err(|e| CpError::IoErrContext(e, context.to_owned()))?;

    Ok(copy_debug)
}
