// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use thiserror::Error;
use uucore::error::UError;
use uucore::translate;

#[derive(Debug, Error)]
pub enum MvError {
    #[error("{}", translate!("mv-error-no-such-file", "path" => .0.clone()))]
    NoSuchFile(String),
    #[error("{}", translate!("mv-error-cannot-stat-not-directory", "path" => .0.clone()))]
    CannotStatNotADirectory(String),
    #[error("{}", translate!("mv-error-same-file", "source" => .0.clone(), "target" => .1.clone()))]
    SameFile(String, String),
    #[error("{}", translate!("mv-error-self-target-subdirectory", "source" => .0.clone(), "target" => .1.clone()))]
    SelfTargetSubdirectory(String, String),
    #[error("{}", translate!("mv-error-directory-to-non-directory", "path" => .0.clone()))]
    DirectoryToNonDirectory(String),
    #[error("{}", translate!("mv-error-non-directory-to-directory", "source" => .0.clone(), "target" => .1.clone()))]
    NonDirectoryToDirectory(String, String),
    #[error("{}", translate!("mv-error-not-directory", "path" => .0.clone()))]
    NotADirectory(String),
    #[error("{}", translate!("mv-error-target-not-directory", "path" => .0.clone()))]
    TargetNotADirectory(String),
    #[error("{}", translate!("mv-error-failed-access-not-directory", "path" => .0.clone()))]
    FailedToAccessNotADirectory(String),
}

impl UError for MvError {}
