// This file is part of the uutils coreutils package.
//
// (c) Maciej Dziardziel <fiedzia@gmail.com>
// (c) Jian Zeng <anonymousknight96 AT gmail.com>
//
// For the full copyright and license information, please view the LICENSE file
// that was distributed with this source code.

// spell-checker:ignore (vars) cvar exitstatus
// spell-checker:ignore (sys/unix) WIFSIGNALED

use libc::{gid_t, pid_t, uid_t};
use std::fmt;
use std::io;
use std::process::Child;
use std::process::ExitStatus as StdExitStatus;
use std::thread;
use std::time::{Duration, Instant};

pub fn geteuid() -> uid_t {
    unsafe { libc::geteuid() }
}

pub fn getegid() -> gid_t {
    unsafe { libc::getegid() }
}

pub fn getgid() -> gid_t {
    unsafe { libc::getgid() }
}

pub fn getuid() -> uid_t {
    unsafe { libc::getuid() }
}

// This is basically sys::unix::process::ExitStatus
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum ExitStatus {
    Code(i32),
    Signal(i32),
}

impl ExitStatus {
    fn from_std_status(status: StdExitStatus) -> Self {
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;

            if let Some(signal) = status.signal() {
                return ExitStatus::Signal(signal);
            }
        }

        // NOTE: this should never fail as we check if the program exited through a signal above
        ExitStatus::Code(status.code().unwrap())
    }

    pub fn success(&self) -> bool {
        match *self {
            ExitStatus::Code(code) => code == 0,
            _ => false,
        }
    }

    pub fn code(&self) -> Option<i32> {
        match *self {
            ExitStatus::Code(code) => Some(code),
            _ => None,
        }
    }

    pub fn signal(&self) -> Option<i32> {
        match *self {
            ExitStatus::Signal(code) => Some(code),
            _ => None,
        }
    }
}

impl fmt::Display for ExitStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ExitStatus::Code(code) => write!(f, "exit code: {}", code),
            ExitStatus::Signal(code) => write!(f, "exit code: {}", code),
        }
    }
}

/// Missing methods for Child objects
pub trait ChildExt {
    /// Send a signal to a Child process.
    fn send_signal(&mut self, signal: usize) -> io::Result<()>;

    /// Wait for a process to finish or return after the specified duration.
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
        // .try_wait() doesn't drop stdin, so we do it manually
        drop(self.stdin.take());

        let start = Instant::now();
        loop {
            if let Some(status) = self.try_wait()? {
                return Ok(Some(ExitStatus::from_std_status(status)));
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
