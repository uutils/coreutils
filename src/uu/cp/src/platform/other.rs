//  * This file is part of the uutils coreutils package.
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.
// spell-checker:ignore reflink
use std::fs;
use std::path::Path;

use quick_error::ResultExt;

use crate::{CopyResult, ReflinkMode, SparseMode};

/// Copies `source` to `dest` for systems without copy-on-write
pub(crate) fn copy_on_write(
    source: &Path,
    dest: &Path,
    reflink_mode: ReflinkMode,
    sparse_mode: SparseMode,
    context: &str,
) -> CopyResult<()> {
    if reflink_mode != ReflinkMode::Never {
        return Err("--reflink is only supported on linux and macOS"
            .to_string()
            .into());
    }
    if sparse_mode != SparseMode::Auto {
        return Err("--sparse is only supported on linux".to_string().into());
    }

    fs::copy(source, dest).context(context)?;

    Ok(())
}
