// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use fs_extra::error::Error as FsXError;
use thiserror::Error;
use uucore::error::UError;

#[derive(Debug, Error)]
pub enum MvError {
    #[error("cannot stat {0}: No such file or directory")]
    NoSuchFile(String),

    #[error("cannot stat {0}: Not a directory")]
    CannotStatNotADirectory(String),

    #[error("{0} and {1} are the same file")]
    SameFile(String, String),

    #[error("cannot move {0} to a subdirectory of itself, {1}")]
    SelfTargetSubdirectory(String, String),

    #[error("cannot overwrite directory {0} with non-directory")]
    DirectoryToNonDirectory(String),

    #[error("cannot overwrite non-directory {1} with directory {0}")]
    NonDirectoryToDirectory(String, String),

    #[error("target {0}: Not a directory")]
    NotADirectory(String),

    #[error("target directory {0}: Not a directory")]
    TargetNotADirectory(String),

    #[error("failed to access {0}: Not a directory")]
    FailedToAccessNotADirectory(String),

    #[error("{0}")]
    FsXError(FsXError),

    #[error("failed to move all files")]
    NotAllFilesMoved,
}

impl UError for MvError {}

impl From<FsXError> for MvError {
    fn from(err: FsXError) -> Self {
        Self::FsXError(err)
    }
}