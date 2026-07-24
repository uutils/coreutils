// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Process helpers, most notably [`ChildExt`], which extends
//! [`std::process::Child`] with signal delivery and a wait-with-timeout.
//!
//! Unix delivers real POSIX signals; Windows emulates them (see the `windows`
//! submodule). Both provide identical trait signatures.

use std::io;
use std::process::ExitStatus;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

/// Missing methods for Child objects
pub trait ChildExt {
    /// Send a signal to a Child process.
    ///
    /// Caller beware: if the process already exited then you may accidentally
    /// send the signal to an unrelated process that recycled the PID.
    fn send_signal(&mut self, signal: usize) -> io::Result<()>;

    /// Send a signal to a process group.
    fn send_signal_group(&mut self, signal: usize) -> io::Result<()>;

    /// Wait for a process to finish or return after the specified duration.
    /// A `timeout` of zero disables the timeout.
    fn wait_or_timeout(
        &mut self,
        timeout: Duration,
        signaled: Option<&AtomicBool>,
    ) -> io::Result<Option<ExitStatus>>;
}

#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use unix::*;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::*;
