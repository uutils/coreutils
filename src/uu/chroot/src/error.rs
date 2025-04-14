// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore NEWROOT Userspec userspec
//! Errors returned by chroot.
use std::io::Error;
use thiserror::Error;
use uucore::display::Quotable;
use uucore::error::UError;
use uucore::libc;

/// Errors that can happen while executing chroot.
#[derive(Debug, Error)]
pub enum ChrootError {
    /// Failed to enter the specified directory.
    #[error("cannot chroot to {dir}: {err}", dir = .0.quote(), err = .1)]
    CannotEnter(String, #[source] Error),

    /// Failed to execute the specified command.
    #[error("failed to run command {cmd}: {err}", cmd = .0.to_string().quote(), err = .1)]
    CommandFailed(String, #[source] Error),

    /// Failed to find the specified command.
    #[error("failed to run command {cmd}: {err}", cmd = .0.to_string().quote(), err = .1)]
    CommandNotFound(String, #[source] Error),

    #[error("--groups parsing failed")]
    GroupsParsingFailed,

    #[error("invalid group: {group}", group = .0.quote())]
    InvalidGroup(String),

    #[error("invalid group list: {list}", list = .0.quote())]
    InvalidGroupList(String),

    /// The new root directory was not given.
    #[error(
        "Missing operand: NEWROOT\nTry '{0} --help' for more information.",
        uucore::execution_phrase()
    )]
    MissingNewRoot,

    #[error("no group specified for unknown uid: {0}")]
    NoGroupSpecified(libc::uid_t),

    /// Failed to find the specified user.
    #[error("invalid user")]
    NoSuchUser,

    /// Failed to find the specified group.
    #[error("invalid group")]
    NoSuchGroup,

    /// The given directory does not exist.
    #[error("cannot change root directory to {dir}: no such directory", dir = .0.quote())]
    NoSuchDirectory(String),

    /// The call to `setgid()` failed.
    #[error("cannot set gid to {gid}: {err}", gid = .0, err = .1)]
    SetGidFailed(String, #[source] Error),

    /// The call to `setgroups()` failed.
    #[error("cannot set groups: {0}")]
    SetGroupsFailed(Error),

    /// The call to `setuid()` failed.
    #[error("cannot set user to {user}: {err}", user = .0.maybe_quote(), err = .1)]
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
