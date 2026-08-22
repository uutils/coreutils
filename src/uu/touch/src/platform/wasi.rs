// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) utimensat

use std::path::PathBuf;

use uucore::translate;

use crate::error::TouchError;

/// WASI has no way to name the file behind stdout.
pub fn pathbuf_from_stdout() -> Result<PathBuf, TouchError> {
    Err(TouchError::UnsupportedPlatformFeature(translate!(
        "touch-error-stdout-unsupported"
    )))
}
