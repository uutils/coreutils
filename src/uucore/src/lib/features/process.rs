// This file is part of the uutils coreutils package.
//
// (c) Maciej Dziardziel <fiedzia@gmail.com>
// (c) Jian Zeng <anonymousknight96 AT gmail.com>
//
// For the full copyright and license information, please view the LICENSE file
// that was distributed with this source code.

// spell-checker:ignore (vars) cvar exitstatus
// spell-checker:ignore (sys/unix) WIFSIGNALED

//! Set of functions to manage IDs

use libc::{gid_t, pid_t, uid_t};
use std::io;
use std::process::Child;
use std::process::ExitStatus;
use std::thread;
use std::time::{Duration, Instant};

// SAFETY: These functions always succeed and return simple integers.

/// `geteuid()` returns the effective user ID of the calling process.
pub fn geteuid() -> uid_t {
    unsafe { libc::geteuid() }
}

/// `getegid()` returns the effective group ID of the calling process.
pub fn getegid() -> gid_t {
    unsafe { libc::getegid() }
}

/// `getgid()` returns the real group ID of the calling process.
pub fn getgid() -> gid_t {
    unsafe { libc::getgid() }
}

/// `getuid()` returns the real user ID of the calling process.
pub fn getuid() -> uid_t {
    unsafe { libc::getuid() }
}

/// Missing methods for Child objects
pub trait ChildExt {
    /// Send a signal to a Child process.
    ///
    /// Caller beware: if the process already exited then you may accidentally
    /// send the signal to an unrelated process that recycled the PID.
    fn send_signal(&mut self, signal: usize) -> io::Result<()>;

    /// Wait for a process to finish or return after the specified duration.
    /// A `timeout` of zero disables the timeout.
    fn wait_or_timeout(&mut self, timeout: Duration) -> io::Result<Option<ExitStatus>>;
}

impl ChildExt for Child {
    fn send_signal(&mut self, signal: usize) -> io::Result<()> {
        if unsafe { libc::kill(self.id() as pid_t, signal as i32) } != 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    fn wait_or_timeout(&mut self, timeout: Duration) -> io::Result<Option<ExitStatus>> {
        if timeout == Duration::from_micros(0) {
            return self.wait().map(Some);
        }
        // .try_wait() doesn't drop stdin, so we do it manually
        drop(self.stdin.take());

        let start = Instant::now();
        loop {
            if let Some(status) = self.try_wait()? {
                return Ok(Some(status));
            }

            if start.elapsed() >= timeout {
                break;
            }

            // XXX: this is kinda gross, but it's cleaner than starting a thread just to wait
            //      (which was the previous solution).  We might want to use a different duration
            //      here as well
            thread::sleep(Duration::from_millis(100));
        }

        Ok(None)
    }
}
