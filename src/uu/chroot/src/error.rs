// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore NEWROOT Userspec userspec
//! Errors returned by chroot.
use std::ffi::OsString;
use std::io::Error;
use std::path::PathBuf;
use thiserror::Error;
use uucore::display::Quotable;
use uucore::error::UError;
use uucore::libc;
use uucore::translate;

/// Errors that can happen while executing chroot.
#[derive(Debug, Error)]
pub enum ChrootError {
    /// Failed to enter the specified directory.
    #[error("{}", translate!("chroot-error-cannot-enter", "dir" => _0.quote(), "err" => _1))]
    CannotEnter(PathBuf, #[source] Error),

    /// Failed to execute the specified command.
    #[error("{}", translate!("chroot-error-command-failed", "cmd" => _0.quote(), "err" => _1))]
    CommandFailed(OsString, #[source] Error),

    /// Failed to find the specified command.
    #[error("{}", translate!("chroot-error-command-not-found", "cmd" => _0.quote(), "err" => _1))]
    CommandNotFound(OsString, #[source] Error),

    #[error("{}", translate!("chroot-error-groups-parsing-failed"))]
    GroupsParsingFailed,

    #[error("{}", translate!("chroot-error-invalid-group", "group" => _0.quote()))]
    InvalidGroup(String),

    #[error("{}", translate!("chroot-error-invalid-group-list", "list" => _0.quote()))]
    InvalidGroupList(String),

    /// The new root directory was not given.
    #[error("{}", translate!("chroot-error-missing-newroot", "util_name" => uucore::execution_phrase()))]
    MissingNewRoot,

    #[error("{}", translate!("chroot-error-no-group-specified", "uid" => _0))]
    NoGroupSpecified(libc::uid_t),

    /// Failed to find the specified user.
    #[error("{}", translate!("chroot-error-no-such-user"))]
    NoSuchUser,

    /// Failed to find the specified group.
    #[error("{}", translate!("chroot-error-no-such-group"))]
    NoSuchGroup,

    /// The given directory does not exist.
    #[error("{}", translate!("chroot-error-no-such-directory", "dir" => _0.quote()))]
    NoSuchDirectory(PathBuf),

    /// The call to `setgid()` failed.
    #[error("{}", translate!("chroot-error-set-gid-failed", "gid" => _0, "err" => _1))]
    SetGidFailed(String, #[source] Error),

    /// The call to `setgroups()` failed.
    #[error("{}", translate!("chroot-error-set-groups-failed", "err" => _0))]
    SetGroupsFailed(Error),

    /// The call to `setuid()` failed.
    #[error("{}", translate!("chroot-error-set-user-failed", "user" => _0.maybe_quote(), "err" => _1))]
    SetUserFailed(String, #[source] Error),
}

impl UError for ChrootError {
    /// 125 if chroot itself fails
    /// 126 if command is found but cannot be invoked
    /// 127 if command cannot be found
    fn code(&self) -> i32 {
        match self {
            Self::CommandFailed(_, _) => 126,
            Self::CommandNotFound(_, _) => 127,
            _ => 125,
        }
    }
}
