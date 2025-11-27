// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Exit status codes produced by `timeout`.

use nix::errno::Errno;
use std::error::Error;
use std::fmt;
use std::io;
use std::os::unix::process::ExitStatusExt;
use std::process::ExitStatus;
use uucore::error::UError;
use uucore::translate;

#[derive(Debug)]
pub(crate) enum TimeoutResult {
    /// The process exited before the timeout expired
    Exited(ExitStatus),
    /// The process was killed after the timeout expired
    TimedOut(ExitStatus),
}

impl TimeoutResult {
    pub(crate) fn to_exit_status(&self, preserve_status: bool) -> ExitStatus {
        match self {
            Self::Exited(status) => *status,
            Self::TimedOut(status) => {
                if preserve_status {
                    if let Some(signal) = status.signal() {
                        // Despite the name of the option, GNU timeout does not actually fully
                        // preserve the status of the child process if it timed out; it just sets
                        // the exit code to the sh conventional value
                        ExitStatus::from_raw((128 + signal) << 8)
                    } else {
                        *status
                    }
                } else {
                    ExitStatus::from_raw(124 << 8)
                }
            }
        }
    }
}

#[derive(Debug)]
pub(crate) enum TimeoutError {
    /// `timeout` itself failed
    Failure(Box<dyn UError>),
    /// Command was found but could not be invoked
    CommandFailedInvocation(io::Error),
    /// Command was not found
    CommandNotFound(io::Error),
}

impl fmt::Display for TimeoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Failure(err) => err.fmt(f),
            Self::CommandFailedInvocation(err) => {
                translate!("timeout-error-failed-to-execute-process", "error" => err).fmt(f)
            }
            Self::CommandNotFound(err) => {
                translate!("timeout-error-failed-to-execute-process", "error" => err).fmt(f)
            }
        }
    }
}

impl Error for TimeoutError {}

impl UError for TimeoutError {
    fn code(&self) -> i32 {
        match self {
            Self::Failure(_) => 125,
            Self::CommandFailedInvocation(_) => 126,
            Self::CommandNotFound(_) => 127,
        }
    }

    fn usage(&self) -> bool {
        match self {
            Self::Failure(err) => err.usage(),
            _ => false,
        }
    }
}

impl From<Box<dyn UError>> for TimeoutError {
    fn from(err: Box<dyn UError>) -> Self {
        Self::Failure(err)
    }
}

impl From<io::Error> for TimeoutError {
    fn from(err: io::Error) -> Self {
        Self::Failure(err.into())
    }
}

impl From<Errno> for TimeoutError {
    fn from(err: Errno) -> Self {
        Self::Failure(err.into())
    }
}
