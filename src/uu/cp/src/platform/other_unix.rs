// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore reflink
use std::fs::{self, File, OpenOptions};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

use uucore::buf_copy;
use uucore::locale::get_message;
use uucore::mode::get_umask;

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
    source_is_fifo: bool,
    source_is_stream: bool,
) -> CopyResult<CopyDebug> {
    if reflink_mode != ReflinkMode::Never {
        return Err(get_message("cp-error-reflink-not-supported")
            .to_string()
            .into());
    }
    if sparse_mode != SparseMode::Auto {
        return Err(get_message("cp-error-sparse-not-supported")
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
        let mode = 0o622 & !get_umask();
        let mut dst_file = OpenOptions::new()
            .create(true)
            .write(true)
            .mode(mode)
            .open(dest)?;

        buf_copy::copy_stream(&mut src_file, &mut dst_file)
            .map_err(|_| std::io::Error::from(std::io::ErrorKind::Other))
            .map_err(|e| CpError::IoErrContext(e, context.to_owned()))?;

        if source_is_fifo {
            dst_file.set_permissions(src_file.metadata()?.permissions())?;
        }
        return Ok(copy_debug);
    }

    fs::copy(source, dest).map_err(|e| CpError::IoErrContext(e, context.to_owned()))?;

    Ok(copy_debug)
}
