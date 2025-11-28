// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (vars) cvar exitstatus cmdline kworker getsid getpid
// spell-checker:ignore (sys/unix) WIFSIGNALED ESRCH
// spell-checker:ignore pgrep pwait snice getpgrp

use libc::{gid_t, pid_t, uid_t};
#[cfg(not(target_os = "redox"))]
use nix::errno::Errno;
use std::io;
use std::process::Child;
use std::process::ExitStatus;
use std::sync::atomic;
use std::sync::atomic::AtomicBool;
use std::thread;
use std::time::{Duration, Instant};

// SAFETY: These functions always succeed and return simple integers.

/// `geteuid()` returns the effective user ID of the calling process.
pub fn geteuid() -> uid_t {
    unsafe { libc::geteuid() }
}

/// `getpgrp()` returns the process group ID of the calling process.
/// It is a trivial wrapper over libc::getpgrp to "hide" the unsafe
pub fn getpgrp() -> pid_t {
    unsafe { libc::getpgrp() }
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

/// `getpid()` returns the pid of the calling process.
pub fn getpid() -> pid_t {
    unsafe { libc::getpid() }
}

/// Check if a process with the given PID is alive.
///
/// Uses `kill(pid, 0)` which sends signal 0 (null signal) to check process existence
/// without actually sending a signal. This is a standard POSIX technique for checking
/// if a process exists.
///
/// Returns `true` if:
/// - The process exists (kill returns 0)
/// - We lack permission to signal the process (errno != ESRCH)
///   This means the process exists but we can't signal it
///
/// Returns `false` only if:
/// - errno is ESRCH (No such process), confirming the process doesn't exist
///
/// PIDs <= 0 are considered alive for compatibility with utmp records that may
/// contain special or invalid PID values.
#[cfg(not(target_os = "openbsd"))]
pub fn pid_is_alive(pid: i32) -> bool {
    if pid <= 0 {
        return true;
    }

    unsafe { libc::kill(pid, 0) == 0 || *libc::__errno_location() != libc::ESRCH }
}

/// `getsid()` returns the session ID of the process with process ID pid.
///
/// If pid is 0, getsid() returns the session ID of the calling process.
///
/// # Error
///
/// - [Errno::EPERM] A process with process ID pid exists, but it is not in the same session as the calling process, and the implementation considers this an error.
/// - [Errno::ESRCH] No process with process ID pid was found.
///
///
/// # Platform
///
/// This function only support standard POSIX implementation platform,
/// so some system such as redox doesn't supported.
#[cfg(not(target_os = "redox"))]
pub fn getsid(pid: i32) -> Result<pid_t, Errno> {
    unsafe {
        let result = libc::getsid(pid);
        if Errno::last() == Errno::UnknownErrno {
            Ok(result)
        } else {
            Err(Errno::last())
        }
    }
}

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

impl ChildExt for Child {
    fn send_signal(&mut self, signal: usize) -> io::Result<()> {
        if unsafe { libc::kill(self.id() as pid_t, signal as i32) } == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }

    fn send_signal_group(&mut self, signal: usize) -> io::Result<()> {
        // Ignore the signal, so we don't go into a signal loop.
        if unsafe { libc::signal(signal as i32, libc::SIG_IGN) } == usize::MAX {
            return Err(io::Error::last_os_error());
        }
        if unsafe { libc::kill(0, signal as i32) } == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }

    fn wait_or_timeout(
        &mut self,
        timeout: Duration,
        signaled: Option<&AtomicBool>,
    ) -> io::Result<Option<ExitStatus>> {
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

            if start.elapsed() >= timeout
                || signaled.is_some_and(|signaled| signaled.load(atomic::Ordering::Relaxed))
            {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(not(target_os = "redox"))]
    fn test_getsid() {
        assert_eq!(
            getsid(getpid()).expect("getsid(getpid)"),
            // zero is a special value for SID.
            // https://pubs.opengroup.org/onlinepubs/9699919799/functions/getsid.html
            getsid(0).expect("getsid(0)")
        );

        // SID never be 0.
        assert!(getsid(getpid()).expect("getsid(getpid)") > 0);

        // This might caused tests failure but the probability is low.
        assert!(getsid(999_999).is_err());
    }
}
