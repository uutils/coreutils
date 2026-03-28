// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (vars) cvar exitstatus cmdline kworker getsid getpid
// spell-checker:ignore (sys/unix) WIFSIGNALED ESRCH
// spell-checker:ignore pgrep pwait snice getpgrp

use libc::{gid_t, pid_t, uid_t};
use rustix::process::{self as rprocess, Pid, Signal};
use std::io;
use std::process::Child;
use std::process::ExitStatus;
use std::sync::atomic;
use std::sync::atomic::AtomicBool;
use std::thread;
use std::time::{Duration, Instant};

use crate::signals::csignal;

/// Convert a raw signal number to a `Signal`, returning an error for invalid values.
fn signal_from_raw(sig: i32) -> io::Result<Signal> {
    if sig > 0 {
        // SAFETY: we've checked the value is positive; the kernel will reject
        // truly invalid signal numbers.
        Ok(unsafe { Signal::from_raw_unchecked(sig) })
    } else {
        Err(io::Error::from_raw_os_error(libc::EINVAL))
    }
}

/// `geteuid()` returns the effective user ID of the calling process.
pub fn geteuid() -> uid_t {
    rprocess::geteuid().as_raw()
}

/// `getpgrp()` returns the process group ID of the calling process.
pub fn getpgrp() -> pid_t {
    rprocess::getpgrp().as_raw_nonzero().get()
}

/// `getegid()` returns the effective group ID of the calling process.
pub fn getegid() -> gid_t {
    rprocess::getegid().as_raw()
}

/// `getgid()` returns the real group ID of the calling process.
pub fn getgid() -> gid_t {
    rprocess::getgid().as_raw()
}

/// `getuid()` returns the real user ID of the calling process.
pub fn getuid() -> uid_t {
    rprocess::getuid().as_raw()
}

/// `getpid()` returns the pid of the calling process.
pub fn getpid() -> pid_t {
    rprocess::getpid().as_raw_nonzero().get()
}

/// `getsid()` returns the session ID of the process with process ID pid.
///
/// If pid is 0, getsid() returns the session ID of the calling process.
///
/// # Error
///
/// - EPERM: A process with process ID pid exists, but it is not in the same session as the calling process, and the implementation considers this an error.
/// - ESRCH: No process with process ID pid was found.
///
/// # Platform
///
/// This function only support standard POSIX implementation platform,
/// so some system such as redox doesn't supported.
#[cfg(not(target_os = "redox"))]
pub fn getsid(pid: i32) -> io::Result<pid_t> {
    let pid = if pid == 0 {
        None
    } else {
        Some(Pid::from_raw(pid).ok_or_else(|| io::Error::from_raw_os_error(libc::EINVAL))?)
    };
    rprocess::getsid(pid)
        .map(|p| p.as_raw_nonzero().get())
        .map_err(Into::into)
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
        let pid = Pid::from_raw(self.id() as pid_t)
            .ok_or_else(|| io::Error::from_raw_os_error(libc::EINVAL))?;
        if signal == 0 {
            rprocess::test_kill_process(pid)?;
        } else {
            let sig = signal_from_raw(signal as i32)?;
            rprocess::kill_process(pid, sig)?;
        }
        Ok(())
    }

    fn send_signal_group(&mut self, signal: usize) -> io::Result<()> {
        // Send signal to our process group (group 0 = caller's group).
        // This matches GNU coreutils behavior: if the child has remained in our
        // process group, it will receive this signal along with all other processes
        // in the group. If the child has created its own process group (via setpgid),
        // it won't receive this group signal, but will have received the direct signal.

        // Signal 0 is special - it just checks if process exists, doesn't send anything.
        // No need to manipulate signal handlers for it.
        if signal == 0 {
            return rprocess::test_kill_current_process_group().map_err(Into::into);
        }

        let sig = signal_from_raw(signal as i32)?;

        // Ignore the signal temporarily so we don't receive it ourselves.
        let old_handler = unsafe {
            csignal::set_signal_handler(signal as libc::c_int, csignal::SigHandler::SigIgn)
        }?;
        let result = rprocess::kill_current_process_group(sig);
        // Restore the old handler
        let _ = unsafe { csignal::set_signal_handler(signal as libc::c_int, old_handler) };
        result.map_err(Into::into)
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
