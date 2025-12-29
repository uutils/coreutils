// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//! Exit status codes produced by `timeout`.
use uucore::error::UError;

/// Enumerates the exit statuses produced by `timeout`.
///
/// Use [`Into::into`] (or [`From::from`]) to convert an enumeration
/// member into a numeric status code. You can also convert into a
/// [`UError`].
///
/// # Examples
///
/// Convert into an [`i32`]:
///
/// ```rust,ignore
/// assert_eq!(i32::from(ExitStatus::CommandTimedOut), 124);
/// ```
pub(crate) enum ExitStatus {
    /// When the child process times out.
    CommandTimedOut,

    /// When `timeout` itself fails.
    TimeoutFailed,

    /// When command is found but cannot be invoked (permission denied, etc.).
    CannotInvoke,

    /// When command cannot be found.
    CommandNotFound,

    /// When a signal is sent to the child process or `timeout` itself.
    SignalSent(usize),

    /// When `SIGTERM` signal received.
    Terminated,
}

impl From<ExitStatus> for i32 {
    fn from(exit_status: ExitStatus) -> Self {
        match exit_status {
            ExitStatus::CommandTimedOut => 124,
            ExitStatus::TimeoutFailed => 125,
            ExitStatus::CannotInvoke => 126,
            ExitStatus::CommandNotFound => 127,
            ExitStatus::SignalSent(s) => 128 + s as Self,
            ExitStatus::Terminated => 143,
        }
    }
}

impl From<ExitStatus> for Box<dyn UError> {
    fn from(exit_status: ExitStatus) -> Self {
        Box::from(i32::from(exit_status))
    }
}
