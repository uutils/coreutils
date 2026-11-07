// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore reflink

use std::fs::{self, File};
use std::path::Path;
use uucore::translate;

use crate::{
    CopyDebug, CopyResult, CpError, OffloadReflinkDebug, ReflinkMode, SparseDebug, SparseMode,
    cannot_open_for_reading,
};

/// Copies `source` to `dest` for systems without copy-on-write
pub(crate) fn copy_on_write(
    source: &Path,
    dest: &Path,
    reflink_mode: ReflinkMode,
    sparse_mode: SparseMode,
    context: &str,
) -> CopyResult<CopyDebug> {
    if reflink_mode == ReflinkMode::Always {
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
    // `fs::copy` doesn't say whether reading the source or writing the
    // destination failed, so look at the source only once it has failed.
    fs::copy(source, dest).map_err(|e| match File::open(source) {
        Err(open_err) => cannot_open_for_reading(open_err, source),
        Ok(_) => CpError::IoErrContext(e, context.to_owned()),
    })?;

    Ok(copy_debug)
}
