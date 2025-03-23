// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//! Errors returned by tac during processing of a file.
use thiserror::Error;

use uucore::display::Quotable;
use uucore::error::UError;

#[derive(Debug, Error)]
pub enum TacError {
    /// A regular expression given by the user is invalid.
    #[error("invalid regular expression: {0}")]
    InvalidRegex(regex::Error),

    /// An argument to tac is invalid.
    #[error("{}: read error: Invalid argument", _0.maybe_quote())]
    InvalidArgument(String),

    /// The specified file is not found on the filesystem.
    #[error("failed to open {} for reading: No such file or directory", _0.quote())]
    FileNotFound(String),

    /// An error reading the contents of a file or stdin.
    ///
    /// The parameters are the name of the file and the underlying
    /// [`std::io::Error`] that caused this error.
    #[error("failed to read from {0}: {1}")]
    ReadError(String, std::io::Error),

    /// An error writing the (reversed) contents of a file or stdin.
    ///
    /// The parameter is the underlying [`std::io::Error`] that caused
    /// this error.
    #[error("failed to write to stdout: {0}")]
    WriteError(std::io::Error),
}

impl UError for TacError {
    fn code(&self) -> i32 {
        1
    }
}
