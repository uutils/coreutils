// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (vars) cvar exitstatus cmdline kworker getsid getpid
// spell-checker:ignore (sys/unix) WIFSIGNALED ESRCH sigtimedwait timespec kqueue kevent EVFILT
// spell-checker:ignore (signals) setpgid PGID sigset EAGAIN ETIMEDOUT ONESHOT eventlist
// spell-checker:ignore pgrep pwait snice getpgrp

use libc::{gid_t, pid_t, uid_t};
#[cfg(not(target_os = "redox"))]
use nix::errno::Errno;
#[cfg(not(any(target_os = "macos", target_os = "freebsd")))]
use nix::sys::signal::SigSet;
use nix::sys::signal::Signal;
use std::io;
use std::process::Child;
use std::process::ExitStatus;
use std::sync::atomic;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

#[cfg(any(target_os = "macos", target_os = "freebsd"))]
use std::time::Instant;

#[cfg(any(target_os = "macos", target_os = "freebsd"))]
use nix::sys::event::{EvFlags, EventFilter, FilterFlag, KEvent, Kqueue};

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
        // Send to our own process group (which the child inherits)
        // After calling setpgid(0, 0), our PGID equals our PID
        if unsafe { libc::kill(-libc::getpid(), signal as i32) } == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "freebsd")))]
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

        // Use sigtimedwait for efficient, precise waiting
        // This suspends the process until either:
        // - SIGCHLD is received (child exited/stopped/continued)
        // - The timeout expires
        // - SIGTERM is received (if signaled parameter is provided)
        //
        // NOTE: Signals must be blocked by the caller BEFORE spawning the child
        // to avoid race conditions. We assume SIGCHLD/SIGTERM are already blocked.

        // Create signal set for signals we want to wait for
        let mut sigset = SigSet::empty();
        sigset.add(Signal::SIGCHLD);
        if signaled.is_some() {
            sigset.add(Signal::SIGTERM);
        }

        // Convert Duration to timespec for sigtimedwait
        // Cap at safe value to avoid platform-specific time_t limits
        const MAX_SAFE_TIMEOUT_SECS: u64 = (i32::MAX / 2) as u64;
        let timeout_spec = libc::timespec {
            tv_sec: timeout.as_secs().min(MAX_SAFE_TIMEOUT_SECS) as libc::time_t,
            tv_nsec: if timeout.as_secs() >= MAX_SAFE_TIMEOUT_SECS {
                0
            } else {
                timeout.subsec_nanos() as libc::c_long
            },
        };

        // Wait for signals with timeout
        // We don't need the siginfo details, so pass NULL for the second parameter
        let ret = unsafe {
            libc::sigtimedwait(
                std::ptr::from_ref(sigset.as_ref()),
                std::ptr::null_mut(),
                std::ptr::from_ref(&timeout_spec),
            )
        };

        match ret {
            // Signal received
            sig if sig > 0 => {
                let signal = Signal::try_from(sig).ok();

                // Check if SIGTERM was received (external termination request)
                if signal == Some(Signal::SIGTERM) {
                    if let Some(flag) = signaled {
                        flag.store(true, atomic::Ordering::Relaxed);
                    }
                    return Ok(None); // Indicate timeout/termination
                }

                // SIGCHLD received - child has changed state (exited, stopped, or continued)
                // Use blocking wait() since we know the child has changed state
                // This ensures we properly reap the child after receiving SIGCHLD
                self.wait().map(Some)
            }
            // Timeout expired (EAGAIN or ETIMEDOUT)
            -1 => {
                let err = Errno::last();
                if err == Errno::EAGAIN || err == Errno::ETIMEDOUT {
                    // Timeout reached, child still running
                    Ok(None)
                } else {
                    // Some other error
                    Err(io::Error::last_os_error())
                }
            }
            // Shouldn't happen
            _ => Err(io::Error::other("unexpected sigtimedwait return value")),
        }
    }

    #[cfg(any(target_os = "macos", target_os = "freebsd"))]
    fn wait_or_timeout(
        &mut self,
        timeout: Duration,
        signaled: Option<&AtomicBool>,
    ) -> io::Result<Option<ExitStatus>> {
        // macOS implementation using kqueue for efficient signal monitoring
        if timeout == Duration::from_micros(0) {
            return self.wait().map(Some);
        }

        // .try_wait() doesn't drop stdin, so we do it manually
        drop(self.stdin.take());

        // Check if child has already exited before setting up kqueue
        // This avoids missing SIGCHLD if child exits very quickly
        if let Some(status) = self.try_wait()? {
            return Ok(Some(status));
        }

        // Create kqueue for signal monitoring
        let kq = Kqueue::new().map_err(|e| io::Error::other(e.to_string()))?;

        // Create events for signals we want to monitor
        let mut changelist = Vec::with_capacity(2);

        // Always monitor SIGCHLD
        changelist.push(KEvent::new(
            Signal::SIGCHLD as usize,
            EventFilter::EVFILT_SIGNAL,
            EvFlags::EV_ADD | EvFlags::EV_ONESHOT,
            FilterFlag::empty(),
            0,
            0,
        ));

        // Optionally monitor SIGTERM
        if signaled.is_some() {
            changelist.push(KEvent::new(
                Signal::SIGTERM as usize,
                EventFilter::EVFILT_SIGNAL,
                EvFlags::EV_ADD | EvFlags::EV_ONESHOT,
                FilterFlag::empty(),
                0,
                0,
            ));
        }

        // Calculate deadline for retry logic
        let deadline = Instant::now().checked_add(timeout);
        // Use None (wait indefinitely) for very large timeouts to avoid platform limits
        // ~34 years is effectively infinite and safe on all platforms
        const MAX_SAFE_TIMEOUT_SECS: u64 = (i32::MAX / 2) as u64;
        let timeout_spec = if timeout.as_secs() >= MAX_SAFE_TIMEOUT_SECS {
            None
        } else {
            Some(libc::timespec {
                tv_sec: timeout.as_secs() as libc::time_t,
                tv_nsec: timeout.subsec_nanos() as libc::c_long,
            })
        };

        // Wait for signal events
        let mut eventlist = vec![KEvent::new(
            0,
            EventFilter::EVFILT_SIGNAL,
            EvFlags::empty(),
            FilterFlag::empty(),
            0,
            0,
        )];

        match kq.kevent(&changelist, &mut eventlist, timeout_spec) {
            Ok(n) if n > 0 => {
                // Signal received - check which one
                let sig_num = eventlist[0].ident() as i32;
                let signal = Signal::try_from(sig_num).ok();

                // Check if SIGTERM was received
                if signal == Some(Signal::SIGTERM) {
                    if let Some(flag) = signaled {
                        flag.store(true, atomic::Ordering::Relaxed);
                    }
                    return Ok(None);
                }

                // SIGCHLD received - reap the child
                self.wait().map(Some)
            }
            Ok(_) => {
                // Timeout expired
                Ok(None)
            }
            Err(Errno::EINTR) => {
                // Interrupted - check if we still have time and retry
                if let Some(deadline) = deadline {
                    let remaining = deadline.saturating_duration_since(Instant::now());
                    if remaining > Duration::ZERO {
                        // Recursively retry with remaining time
                        self.wait_or_timeout(remaining, signaled)
                    } else {
                        Ok(None)
                    }
                } else {
                    // No deadline (timeout overflowed), just retry with original timeout
                    self.wait_or_timeout(timeout, signaled)
                }
            }
            Err(e) => Err(io::Error::other(e.to_string())),
        }
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
