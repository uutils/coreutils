//  * This file is part of the uutils coreutils package.
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.
// spell-checker:ignore NEWROOT Userspec userspec
//! Errors returned by chroot.
use std::fmt::Display;
use std::io::Error;
use uucore::display::Quotable;
use uucore::error::UError;

/// Errors that can happen while executing chroot.
#[derive(Debug)]
pub enum ChrootError {
    /// Failed to enter the specified directory.
    CannotEnter(String, Error),

    /// Failed to execute the specified command.
    CommandFailed(String, Error),

    /// The given user and group specification was invalid.
    InvalidUserspec(String),

    /// The new root directory was not given.
    MissingNewRoot,

    /// Failed to find the specified group.
    NoSuchGroup(String),

    /// The given directory does not exist.
    NoSuchDirectory(String),

    /// The call to `setgid()` failed.
    SetGidFailed(String, Error),

    /// The call to `setgroups()` failed.
    SetGroupsFailed(Error),

    /// The call to `setuid()` failed.
    SetUserFailed(String, Error),
}

impl std::error::Error for ChrootError {}

impl UError for ChrootError {
    // TODO: Exit status:
    // 125 if chroot itself fails
    // 126 if command is found but cannot be invoked
    // 127 if command cannot be found
    fn code(&self) -> i32 {
        1
    }
}

impl Display for ChrootError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ChrootError::CannotEnter(s, e) => write!(f, "cannot chroot to {}: {}", s.quote(), e,),
            ChrootError::CommandFailed(s, e) => {
                write!(f, "failed to run command {}: {}", s.to_string().quote(), e,)
            }
            ChrootError::InvalidUserspec(s) => write!(f, "invalid userspec: {}", s.quote(),),
            ChrootError::MissingNewRoot => write!(
                f,
                "Missing operand: NEWROOT\nTry '{} --help' for more information.",
                uucore::execution_phrase(),
            ),
            ChrootError::NoSuchGroup(s) => write!(f, "no such group: {}", s.maybe_quote(),),
            ChrootError::NoSuchDirectory(s) => write!(
                f,
                "cannot change root directory to {}: no such directory",
                s.quote(),
            ),
            ChrootError::SetGidFailed(s, e) => write!(f, "cannot set gid to {}: {}", s, e),
            ChrootError::SetGroupsFailed(e) => write!(f, "cannot set groups: {}", e),
            ChrootError::SetUserFailed(s, e) => {
                write!(f, "cannot set user to {}: {}", s.maybe_quote(), e)
            }
        }
    }
}
