// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::io;
use std::path::Path;
use uucore::display::Quotable;
use uucore::error::{FromIo, UError, USimpleError};
use uucore::translate;

#[derive(Debug, thiserror::Error)]
pub enum MvError {
    #[error("{}", translate!("mv-error-no-such-file", "path" => .0))]
    NoSuchFile(String),
    #[error("{}", translate!("mv-error-cannot-stat-not-directory", "path" => .0))]
    CannotStatNotADirectory(String),
    #[error("{}", translate!("mv-error-same-file", "source" => .0, "target" => .1))]
    SameFile(String, String),
    #[error("{}", translate!("mv-error-self-target-subdirectory", "source" => .0, "target" => .1))]
    SelfTargetSubdirectory(String, String),
    #[error("{}", translate!("mv-error-directory-to-non-directory", "path" => .0))]
    DirectoryToNonDirectory(String),
    #[error("{}", translate!("mv-error-non-directory-to-directory", "source" => .0, "target" => .1))]
    NonDirectoryToDirectory(String, String),
    #[error("{}", translate!("mv-error-not-directory", "path" => .0))]
    NotADirectory(String),
    #[error("{}", translate!("mv-error-target-not-directory", "path" => .0))]
    TargetNotADirectory(String),
    #[error("{}", translate!("mv-error-failed-access-not-directory", "path" => .0))]
    FailedToAccessNotADirectory(String),
}

impl UError for MvError {}

#[derive(Debug)]
pub enum RenameError {
    /// A raw I/O error from `fs::rename` (or general failure) that should be
    /// contextualized as "cannot move 'source' to 'target': {err}" or
    /// "cannot overwrite 'target': Directory not empty".
    Io(io::Error),
    /// A specific diagnostic that already includes the file path and operation,
    /// such as "cannot open 'src' for reading: Permission denied" or
    /// "error writing 'dst': No space left on device".
    Formatted(String),
}

impl std::fmt::Display for RenameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "{e}"),
            Self::Formatted(s) => write!(f, "{s}"),
        }
    }
}

impl std::error::Error for RenameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Formatted(_) => None,
        }
    }
}

impl From<io::Error> for RenameError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<RenameError> for io::Error {
    fn from(err: RenameError) -> Self {
        match err {
            RenameError::Io(e) => e,
            RenameError::Formatted(s) => Self::other(s),
        }
    }
}

impl RenameError {
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Io(e) => e.to_string().is_empty(),
            Self::Formatted(s) => s.is_empty(),
        }
    }

    pub fn into_uerror(self, source: &Path, target: &Path) -> Box<dyn UError> {
        match self {
            Self::Formatted(s) => USimpleError::new(1, s),
            Self::Io(e) if e.to_string().is_empty() => USimpleError::new(1, ""),
            Self::Io(e) => {
                let message = if e.kind() == io::ErrorKind::DirectoryNotEmpty {
                    translate!(
                        "mv-error-cannot-overwrite-non-empty-directory",
                        "target" => target.quote()
                    )
                } else {
                    translate!(
                        "mv-error-cannot-move",
                        "source" => source.quote(),
                        "target" => target.quote()
                    )
                };
                e.map_err_context(|| message)
            }
        }
    }
}
