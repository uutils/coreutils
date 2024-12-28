// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use crate::error::UError;

/// Error types used by buffer-copying functions from the `buf_copy` module.
#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    WriteError(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::WriteError(msg) => write!(f, "splice() write error: {}", msg),
            Error::Io(err) => write!(f, "I/O error: {}", err),
        }
    }
}

impl std::error::Error for Error {}

impl UError for Error {
    fn code(&self) -> i32 {
        1
    }

    fn usage(&self) -> bool {
        false
    }
}
