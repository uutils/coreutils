// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (misc) uioerror

use std::error::Error;
use std::fmt::{Display, Formatter, Result};
use std::path::PathBuf;

use filetime::FileTime;
use uucore::display::Quotable;
use uucore::error::{UError, UIoError};

#[derive(Debug)]
pub enum TouchError {
    InvalidDateFormat(String),

    /// The source time couldn't be converted to a [chrono::DateTime]
    InvalidFiletime(FileTime),

    /// The reference file's attributes could not be found or read
    ReferenceFileInaccessible(PathBuf, std::io::Error),

    /// An error getting a path to stdout on Windows
    WindowsStdoutPathError(String),

    /// An error encountered on a specific file
    TouchFileError {
        path: PathBuf,
        index: usize,
        error: Box<dyn UError>,
    },
}

impl Error for TouchError {}
impl UError for TouchError {}
impl Display for TouchError {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match self {
            Self::InvalidDateFormat(s) => write!(f, "Unable to parse date: {}", s),
            Self::InvalidFiletime(time) => write!(
                f,
                "Source has invalid access or modification time: {}",
                time,
            ),
            Self::ReferenceFileInaccessible(path, err) => {
                write!(
                    f,
                    "failed to get attributes of {}: {}",
                    path.quote(),
                    to_uioerror(err)
                )
            }
            Self::WindowsStdoutPathError(code) => {
                write!(f, "GetFinalPathNameByHandleW failed with code {}", code)
            }
            Self::TouchFileError { error, .. } => write!(f, "{}", error),
        }
    }
}

fn to_uioerror(err: &std::io::Error) -> UIoError {
    let copy = if let Some(code) = err.raw_os_error() {
        std::io::Error::from_raw_os_error(code)
    } else {
        std::io::Error::from(err.kind())
    };
    UIoError::from(copy)
}
