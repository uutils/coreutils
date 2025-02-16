// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (misc) uioerror
use filetime::FileTime;
use std::path::PathBuf;
use thiserror::Error;
use uucore::display::Quotable;
use uucore::error::{UError, UIoError};

#[derive(Debug, Error)]
pub enum TouchError {
    #[error("Unable to parse date: {0}")]
    InvalidDateFormat(String),

    /// The source time couldn't be converted to a [chrono::DateTime]
    #[error("Source has invalid access or modification time: {0}")]
    InvalidFiletime(FileTime),

    /// The reference file's attributes could not be found or read
    #[error("failed to get attributes of {}: {}", .0.quote(), to_uioerror(.1))]
    ReferenceFileInaccessible(PathBuf, std::io::Error),

    /// An error getting a path to stdout on Windows
    #[error("GetFinalPathNameByHandleW failed with code {0}")]
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
