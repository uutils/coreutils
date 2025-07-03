// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (misc) uioerror
use filetime::FileTime;
use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;
use uucore::display::Quotable;
use uucore::error::{UError, UIoError};
use uucore::locale::get_message_with_args;

#[derive(Debug, Error)]
pub enum TouchError {
    #[error("{}", get_message_with_args("touch-error-unable-to-parse-date", HashMap::from([("date".to_string(), .0.clone())])))]
    InvalidDateFormat(String),

    /// The source time couldn't be converted to a [`chrono::DateTime`]
    #[error("{}", get_message_with_args("touch-error-invalid-filetime", HashMap::from([("time".to_string(), .0.to_string())])))]
    InvalidFiletime(FileTime),

    /// The reference file's attributes could not be found or read
    #[error("{}", get_message_with_args("touch-error-reference-file-inaccessible", HashMap::from([("path".to_string(), .0.quote().to_string()), ("error".to_string(), to_uioerror(.1).to_string())])))]
    ReferenceFileInaccessible(PathBuf, std::io::Error),

    /// An error getting a path to stdout on Windows
    #[error("{}", get_message_with_args("touch-error-windows-stdout-path-failed", HashMap::from([("code".to_string(), .0.clone())])))]
    WindowsStdoutPathError(String),

    /// An error encountered on a specific file
    #[error("{error}")]
    TouchFileError {
        path: PathBuf,
        index: usize,
        error: Box<dyn UError>,
    },
}

fn to_uioerror(err: &std::io::Error) -> UIoError {
    let copy = if let Some(code) = err.raw_os_error() {
        std::io::Error::from_raw_os_error(code)
    } else {
        std::io::Error::from(err.kind())
    };
    UIoError::from(copy)
}

impl UError for TouchError {}
