// This file is part of the uutils coreutils package.
//
// (c) Maciej Dziardziel <fiedzia@gmail.com>
// (c) Jian Zeng <anonymousknight96 AT gmail.com>
//
// For the full copyright and license information, please view the LICENSE file
// that was distributed with this source code.
//

use super::libc;
use libc::{c_int, pid_t, uid_t, gid_t};
use std::fmt;
use std::io;
use std::process::Child;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};

pub fn geteuid() -> uid_t {
    unsafe {
        libc::geteuid()
    }
}

pub fn getegid() -> gid_t {
    unsafe {
        libc::getegid()
    }
}

pub fn getgid() -> gid_t {
    unsafe {
        libc::getgid()
    }
}

pub fn getuid() -> uid_t {
    unsafe {
        libc::getuid()
    }
}

// This is basically sys::unix::process::ExitStatus
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum ExitStatus {
    Code(i32),
    Signal(i32),
}

impl ExitStatus {
    fn from_status(status: c_int) -> ExitStatus {
        if status & 0x7F != 0 {
            // WIFSIGNALED(status)
            ExitStatus::Signal(status & 0x7F)
        } else {
            ExitStatus::Code(status & 0xFF00 >> 8)
        }
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
        // The result will be written to that Option, protected by a Mutex
        // Then the Condvar will be signaled
        let state = Arc::new((Mutex::new(Option::None::<io::Result<ExitStatus>>), Condvar::new()));

        // Start the waiting thread
        let state_th = state.clone();
        let pid_th = self.id();
        thread::spawn(move || {
            let &(ref lock_th, ref cvar_th) = &*state_th;
            // Child::wait() would need a &mut to self, can't use that...
            // use waitpid() directly, with our own ExitStatus
            let mut status: c_int = 0;
            let r = unsafe { libc::waitpid(pid_th as i32, &mut status, 0) };
            // Fill the Option and notify on the Condvar
            let mut exitstatus_th = lock_th.lock().unwrap();
            if r != pid_th as c_int {
                *exitstatus_th = Some(Err(io::Error::last_os_error()));
            } else {
                let s = ExitStatus::from_status(status);
                *exitstatus_th = Some(Ok(s));
            }
            cvar_th.notify_one();
        });

        // Main thread waits
        let &(ref lock, ref cvar) = &*state;
        let mut exitstatus = lock.lock().unwrap();
        // Condvar::wait_timeout_ms() can wake too soon, in this case wait again
        let start = Instant::now();
        loop {
            if let Some(exitstatus) = exitstatus.take() {
                return exitstatus.map(Some);
            }
            if start.elapsed() >= timeout {
                return Ok(None);
            }
            let cvar_timeout = timeout - start.elapsed();
            exitstatus = cvar.wait_timeout(exitstatus, cvar_timeout).unwrap().0;
        }
    }
}
