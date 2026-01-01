// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//! Errors returned by tac during processing of a file.

use std::ffi::OsString;
use thiserror::Error;
use uucore::display::Quotable;
use uucore::error::UError;
use uucore::translate;

#[derive(Debug, Error)]
pub enum TacError {
    /// A regular expression given by the user is invalid.
    #[error("{}", translate!("tac-error-invalid-regex", "error" => .0))]
    InvalidRegex(regex::Error),
    /// The argument to tac is a directory.
    #[error("{}", translate!("tac-error-invalid-directory-argument", "argument" => .0.maybe_quote()))]
    InvalidDirectoryArgument(OsString),
    /// The specified file is not found on the filesystem.
    #[error("{}", translate!("tac-error-file-not-found", "filename" => .0.quote()))]
    FileNotFound(OsString),
    /// An error reading the contents of a file or stdin.
    ///
    /// The parameters are the name of the file and the underlying
    /// [`std::io::Error`] that caused this error.
    #[error("{}", translate!("tac-error-read-error", "filename" => .0.quote(), "error" => .1))]
    ReadError(OsString, std::io::Error),
    /// An error writing the (reversed) contents of a file or stdin.
    ///
    /// The parameter is the underlying [`std::io::Error`] that caused
    /// this error.
    #[error("{}", translate!("tac-error-write-error", "error" => .0))]
    WriteError(std::io::Error),
}

impl UError for TacError {
    fn code(&self) -> i32 {
        1
    }
}
