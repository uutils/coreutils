//  * This file is part of the uutils coreutils package.
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.
//! Errors returned by tac during processing of a file.
use std::error::Error;
use std::fmt::Display;

use uucore::display::Quotable;
use uucore::error::UError;

#[derive(Debug)]
pub enum TacError {
    /// A regular expression given by the user is invalid.
    InvalidRegex(regex::Error),

    /// An argument to tac is invalid.
    InvalidArgument(String),

    /// The specified file is not found on the filesystem.
    FileNotFound(String),

    /// An error reading the contents of a file or stdin.
    ///
    /// The parameters are the name of the file and the underlying
    /// [`std::io::Error`] that caused this error.
    ReadError(String, std::io::Error),

    /// An error writing the (reversed) contents of a file or stdin.
    ///
    /// The parameter is the underlying [`std::io::Error`] that caused
    /// this error.
    WriteError(std::io::Error),
}

impl UError for TacError {
    fn code(&self) -> i32 {
        1
    }
}

impl Error for TacError {}

impl Display for TacError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TacError::InvalidRegex(e) => write!(f, "invalid regular expression: {}", e),
            TacError::InvalidArgument(s) => {
                write!(f, "{}: read error: Invalid argument", s.maybe_quote())
            }
            TacError::FileNotFound(s) => write!(
                f,
                "failed to open {} for reading: No such file or directory",
                s.quote()
            ),
            TacError::ReadError(s, e) => write!(f, "failed to read from {}: {}", s, e),
            TacError::WriteError(e) => write!(f, "failed to write to stdout: {}", e),
        }
    }
}
