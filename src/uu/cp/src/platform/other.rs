// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore reflink

use quick_error::ResultExt;
use std::fs;
use std::path::Path;

#[cfg(not(windows))]
use uucore::mode::get_umask;

use crate::{CopyDebug, CopyResult, OffloadReflinkDebug, ReflinkMode, SparseDebug, SparseMode};

#[cfg(unix)]
use std::fs::{File, OpenOptions};
#[cfg(unix)]
use std::io;
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

/// Copies `source` to `dest` for systems without copy-on-write
pub(crate) fn copy_on_write(
    source: &Path,
    dest: &Path,
    reflink_mode: ReflinkMode,
    sparse_mode: SparseMode,
    context: &str,
    #[cfg(unix)] source_is_stream: bool,
    #[cfg(unix)] source_is_fifo: bool,
) -> CopyResult<CopyDebug> {
    if reflink_mode != ReflinkMode::Never {
        return Err("--reflink is only supported on linux and macOS"
            .to_string()
            .into());
    }
    if sparse_mode != SparseMode::Auto {
        return Err("--sparse is only supported on linux".to_string().into());
    }
    let copy_debug = CopyDebug {
        offload: OffloadReflinkDebug::Unsupported,
        reflink: OffloadReflinkDebug::Unsupported,
        sparse_detection: SparseDebug::Unsupported,
    };

    #[cfg(unix)]
    if source_is_stream {
        let mut src_file = File::open(source)?;
        let mode = 0o622 & !get_umask();
        let mut dst_file = OpenOptions::new()
            .create(true)
            .write(true)
            .mode(mode)
            .open(dest)?;
        io::copy(&mut src_file, &mut dst_file).context(context)?;

        if source_is_fifo {
            dst_file.set_permissions(src_file.metadata()?.permissions())?;
        }
        return Ok(copy_debug);
    }

    fs::copy(source, dest).context(context)?;

    Ok(copy_debug)
}
