// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Errors returned by tac during processing of a file.

use regex_automata::meta::BuildError;
use std::ffi::OsString;
use uucore::display::Quotable;
use uucore::error::{UError, strip_errno};
use uucore::{translate, translate_text};

fn regex_error_message(err: &BuildError) -> String {
    if let Some(limit) = err.size_limit() {
        format!("Compiled regex exceeds size limit of {limit} bytes.")
    } else if let Some(syntax_err) = err.syntax_error() {
        syntax_err.to_string()
    } else {
        err.to_string()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TacError {
    /// A regular expression given by the user is invalid.
    #[error("{}", translate!("tac-error-invalid-regex", "error" => regex_error_message(.0)))]
    InvalidRegex(BuildError),
    /// An error opening a file for reading.
    ///
    /// The parameters are the name of the file and the underlying
    /// [`std::io::Error`] that caused this error.
    #[error("{}", translate_text!("tac-error-open-error", "filename" => .0.quote(), "error" => strip_errno(.1)))]
    OpenError(OsString, std::io::Error),
    /// An error reading the contents of a file or stdin.
    ///
    /// The parameters are the name of the file and the underlying
    /// [`std::io::Error`] that caused this error.
    #[error("{}", translate_text!("tac-error-read-error", "filename" => .0.maybe_quote(), "error" => strip_errno(.1)))]
    ReadError(OsString, std::io::Error),
    /// An error writing the (reversed) contents of a file or stdin.
    ///
    /// The parameter is the underlying [`std::io::Error`] that caused
    /// this error.
    #[error("{}", translate!("tac-error-write-error", "error" => strip_errno(.0)))]
    WriteError(std::io::Error),
}

impl UError for TacError {
    fn code(&self) -> i32 {
        1
    }
}
