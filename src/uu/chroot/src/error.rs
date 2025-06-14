// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore NEWROOT Userspec userspec
//! Errors returned by chroot.
use std::collections::HashMap;
use std::io::Error;
use thiserror::Error;
use uucore::display::Quotable;
use uucore::error::UError;
use uucore::libc;
use uucore::locale::{get_message, get_message_with_args};

/// Errors that can happen while executing chroot.
#[derive(Debug, Error)]
pub enum ChrootError {
    /// Failed to enter the specified directory.
    #[error("{}", get_message_with_args("chroot-error-cannot-enter", HashMap::from([
        ("dir".to_string(), _0.quote().to_string()),
        ("err".to_string(), _1.to_string())
    ])))]
    CannotEnter(String, #[source] Error),

    /// Failed to execute the specified command.
    #[error("{}", get_message_with_args("chroot-error-command-failed", HashMap::from([
        ("cmd".to_string(), _0.to_string().quote().to_string()),
        ("err".to_string(), _1.to_string())
    ])))]
    CommandFailed(String, #[source] Error),

    /// Failed to find the specified command.
    #[error("{}", get_message_with_args("chroot-error-command-not-found", HashMap::from([
        ("cmd".to_string(), _0.to_string().quote().to_string()),
        ("err".to_string(), _1.to_string())
    ])))]
    CommandNotFound(String, #[source] Error),

    #[error("{}", get_message("chroot-error-groups-parsing-failed"))]
    GroupsParsingFailed,

    #[error("{}", get_message_with_args("chroot-error-invalid-group", HashMap::from([
        ("group".to_string(), _0.quote().to_string())
    ])))]
    InvalidGroup(String),

    #[error("{}", get_message_with_args("chroot-error-invalid-group-list", HashMap::from([
        ("list".to_string(), _0.quote().to_string())
    ])))]
    InvalidGroupList(String),

    /// The new root directory was not given.
    #[error("{}", get_message_with_args("chroot-error-missing-newroot", HashMap::from([
        ("util_name".to_string(), uucore::execution_phrase().to_string())
    ])))]
    MissingNewRoot,

    #[error("{}", get_message_with_args("chroot-error-no-group-specified", HashMap::from([
        ("uid".to_string(), _0.to_string())
    ])))]
    NoGroupSpecified(libc::uid_t),

    /// Failed to find the specified user.
    #[error("{}", get_message("chroot-error-no-such-user"))]
    NoSuchUser,

    /// Failed to find the specified group.
    #[error("{}", get_message("chroot-error-no-such-group"))]
    NoSuchGroup,

    /// The given directory does not exist.
    #[error("{}", get_message_with_args("chroot-error-no-such-directory", HashMap::from([
        ("dir".to_string(), _0.quote().to_string())
    ])))]
    NoSuchDirectory(String),

    /// The call to `setgid()` failed.
    #[error("{}", get_message_with_args("chroot-error-set-gid-failed", HashMap::from([
        ("gid".to_string(), _0.to_string()),
        ("err".to_string(), _1.to_string())
    ])))]
    SetGidFailed(String, #[source] Error),

    /// The call to `setgroups()` failed.
    #[error("{}", get_message_with_args("chroot-error-set-groups-failed", HashMap::from([
        ("err".to_string(), _0.to_string())
    ])))]
    SetGroupsFailed(Error),

    /// The call to `setuid()` failed.
    #[error("{}", get_message_with_args("chroot-error-set-user-failed", HashMap::from([
        ("user".to_string(), _0.maybe_quote().to_string()),
        ("err".to_string(), _1.to_string())
    ])))]
    SetUserFailed(String, #[source] Error),
}

impl UError for ChrootError {
    // 125 if chroot itself fails
    // 126 if command is found but cannot be invoked
    // 127 if command cannot be found
    fn code(&self) -> i32 {
        match self {
            Self::CommandFailed(_, _) => 126,
            Self::CommandNotFound(_, _) => 127,
            _ => 125,
        }
    }
}
