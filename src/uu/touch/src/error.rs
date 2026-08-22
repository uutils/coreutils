// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (misc) uioerror
use std::path::PathBuf;
use std::time::SystemTime;
use thiserror::Error;
use uucore::display::Quotable;
use uucore::error::{UError, UIoError};
use uucore::translate;

#[derive(Debug, Error)]
pub enum TouchError {
    #[error("{}", translate!("touch-error-unable-to-parse-date", "date" => .0.clone()))]
    InvalidDateFormat(String),

    /// The source time couldn't be converted to a [`jiff::Zoned`]
    #[error("{}", translate!("touch-error-invalid-filetime", "time" => format_time(*.0)))]
    InvalidFiletime(SystemTime),

    /// The reference file's attributes could not be found or read
    #[error("{}", translate!("touch-error-reference-file-inaccessible", "path" => .0.quote(), "error" => to_uioerror(.1)))]
    ReferenceFileInaccessible(PathBuf, std::io::Error),

    /// An error getting a path to stdout on Windows
    #[error("{}", translate!("touch-error-windows-stdout-path-failed", "code" => .0.clone()))]
    WindowsStdoutPathError(String),

    /// A feature that is not available on the current platform
    #[error("{0}")]
    UnsupportedPlatformFeature(String),

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

fn format_time(time: SystemTime) -> String {
    match time.duration_since(SystemTime::UNIX_EPOCH) {
        Ok(d) => format!("{}.{:09}s", d.as_secs(), d.subsec_nanos()),
        Err(e) => {
            let d = e.duration();
            let subsec = d.subsec_nanos();
            let (sec_offset, nanos) = if subsec == 0 {
                (0, 0)
            } else {
                (1, 1_000_000_000 - subsec)
            };
            format!("{}.{nanos:09}s", -(d.as_secs() as i64 + sec_offset))
        }
    }
}

impl UError for TouchError {}
