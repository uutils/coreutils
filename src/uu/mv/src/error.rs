// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use std::collections::HashMap;
use thiserror::Error;
use uucore::error::UError;
use uucore::locale::get_message_with_args;

#[derive(Debug, Error)]
pub enum MvError {
    #[error("{}", get_message_with_args("mv-error-no-such-file", HashMap::from([("path".to_string(), .0.clone())])))]
    NoSuchFile(String),
    #[error("{}", get_message_with_args("mv-error-cannot-stat-not-directory", HashMap::from([("path".to_string(), .0.clone())])))]
    CannotStatNotADirectory(String),
    #[error("{}", get_message_with_args("mv-error-same-file", HashMap::from([("source".to_string(), .0.clone()), ("target".to_string(), .1.clone())])))]
    SameFile(String, String),
    #[error("{}", get_message_with_args("mv-error-self-target-subdirectory", HashMap::from([("source".to_string(), .0.clone()), ("target".to_string(), .1.clone())])))]
    SelfTargetSubdirectory(String, String),
    #[error("{}", get_message_with_args("mv-error-directory-to-non-directory", HashMap::from([("path".to_string(), .0.clone())])))]
    DirectoryToNonDirectory(String),
    #[error("{}", get_message_with_args("mv-error-non-directory-to-directory", HashMap::from([("source".to_string(), .0.clone()), ("target".to_string(), .1.clone())])))]
    NonDirectoryToDirectory(String, String),
    #[error("{}", get_message_with_args("mv-error-not-directory", HashMap::from([("path".to_string(), .0.clone())])))]
    NotADirectory(String),
    #[error("{}", get_message_with_args("mv-error-target-not-directory", HashMap::from([("path".to_string(), .0.clone())])))]
    TargetNotADirectory(String),
    #[error("{}", get_message_with_args("mv-error-failed-access-not-directory", HashMap::from([("path".to_string(), .0.clone())])))]
    FailedToAccessNotADirectory(String),
}

impl UError for MvError {}
