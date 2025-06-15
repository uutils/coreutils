// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//! Errors returned by tac during processing of a file.

use std::collections::HashMap;
use thiserror::Error;
use uucore::display::Quotable;
use uucore::error::UError;
use uucore::locale::get_message_with_args;

#[derive(Debug, Error)]
pub enum TacError {
    /// A regular expression given by the user is invalid.
    #[error("{}", get_message_with_args("tac-error-invalid-regex", HashMap::from([("error".to_string(), .0.to_string())])))]
    InvalidRegex(regex::Error),
    /// An argument to tac is invalid.
    #[error("{}", get_message_with_args("tac-error-invalid-argument", HashMap::from([("argument".to_string(), .0.maybe_quote().to_string())])))]
    InvalidArgument(String),
    /// The specified file is not found on the filesystem.
    #[error("{}", get_message_with_args("tac-error-file-not-found", HashMap::from([("filename".to_string(), .0.quote().to_string())])))]
    FileNotFound(String),
    /// An error reading the contents of a file or stdin.
    ///
    /// The parameters are the name of the file and the underlying
    /// [`std::io::Error`] that caused this error.
    #[error("{}", get_message_with_args("tac-error-read-error", HashMap::from([("filename".to_string(), .0.clone()), ("error".to_string(), .1.to_string())])))]
    ReadError(String, std::io::Error),
    /// An error writing the (reversed) contents of a file or stdin.
    ///
    /// The parameter is the underlying [`std::io::Error`] that caused
    /// this error.
    #[error("{}", get_message_with_args("tac-error-write-error", HashMap::from([("error".to_string(), .0.to_string())])))]
    WriteError(std::io::Error),
}

impl UError for TacError {
    fn code(&self) -> i32 {
        1
    }
}
