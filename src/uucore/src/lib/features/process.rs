// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (vars) cvar exitstatus cmdline kworker getsid getpid
// spell-checker:ignore (sys/unix) WIFSIGNALED ESRCH
// spell-checker:ignore pgrep pwait snice getpgrp
// spell-checker:ignore sigwait KTIME timeval itimerval setitimer itimer timerid
// spell-checker:ignore sigevent sigev sigval itimerspec signo clockid sevp

use libc::{gid_t, pid_t, uid_t};
#[cfg(not(target_os = "redox"))]
use nix::errno::Errno;
use nix::sys::signal::{self as nix_signal, SigHandler, SigSet, Signal};
use nix::unistd::Pid;
use rustix::process::Signal as RixSignal;
use std::io;
use std::process::Child;
use std::process::ExitStatus;
use std::time::{Duration, Instant};
use timer::Timer;

/// `geteuid()` returns the effective user ID of the calling process.
pub fn geteuid() -> uid_t {
    nix::unistd::geteuid().as_raw()
}

/// `getpgrp()` returns the process group ID of the calling process.
/// It is a trivial wrapper over nix::unistd::getpgrp.
pub fn getpgrp() -> pid_t {
    nix::unistd::getpgrp().as_raw()
}

/// `getegid()` returns the effective group ID of the calling process.
pub fn getegid() -> gid_t {
    nix::unistd::getegid().as_raw()
}

/// `getgid()` returns the real group ID of the calling process.
pub fn getgid() -> gid_t {
    nix::unistd::getgid().as_raw()
}

/// `getuid()` returns the real user ID of the calling process.
pub fn getuid() -> uid_t {
    rustix::process::getuid().as_raw()
}

/// `getpid()` returns the pid of the calling process.
pub fn getpid() -> pid_t {
    nix::unistd::getpid().as_raw()
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
    let pid = if pid == 0 {
        None
    } else {
        Some(Pid::from_raw(pid))
    };
    nix::unistd::getsid(pid).map(Pid::as_raw)
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
    fn wait_or_timeout(&mut self, timeout: Duration, ignore_term: bool) -> io::Result<TimeoutRet>;
}

pub enum TimeoutRet {
    Interrupted(RixSignal),
    Exited(ExitStatus),
    TimedOut,
}

impl ChildExt for Child {
    fn send_signal(&mut self, signal: usize) -> io::Result<()> {
        let pid = Pid::from_raw(self.id() as pid_t);
        let result = if signal == 0 {
            nix_signal::kill(pid, None)
        } else {
            let signal = Signal::try_from(signal as i32)
                .map_err(|_| io::Error::from_raw_os_error(libc::EINVAL))?;
            nix_signal::kill(pid, Some(signal))
        };
        result.map_err(|e| io::Error::from_raw_os_error(e as i32))
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
            return nix_signal::kill(Pid::from_raw(0), None)
                .map_err(|e| io::Error::from_raw_os_error(e as i32));
        }

        let signal = Signal::try_from(signal as i32)
            .map_err(|_| io::Error::from_raw_os_error(libc::EINVAL))?;

        // Ignore the signal temporarily so we don't receive it ourselves.
        let old_handler = unsafe { nix_signal::signal(signal, SigHandler::SigIgn) }
            .map_err(|e| io::Error::from_raw_os_error(e as i32))?;
        let result = nix_signal::kill(Pid::from_raw(0), Some(signal));
        // Restore the old handler
        let _ = unsafe { nix_signal::signal(signal, old_handler) };
        result.map_err(|e| io::Error::from_raw_os_error(e as i32))
    }

    fn wait_or_timeout(&mut self, timeout: Duration, ignore_term: bool) -> io::Result<TimeoutRet> {
        if timeout == Duration::from_micros(0) {
            return self.wait().map(TimeoutRet::Exited);
        }
        // .try_wait() doesn't drop stdin, so we do it manually
        drop(self.stdin.take());

        // Waits continuously whenever we receive an external SIGCHLD or
        // we SIGTERM when we are ignoring them.
        let start = Instant::now();
        let mut remaining = timeout;
        let mut timer = Timer::new()?;
        loop {
            match timer.timed_sigwait(remaining) {
                Ok(None) => break Ok(TimeoutRet::TimedOut),
                Ok(Some(Signal::SIGCHLD)) => {
                    if let Some(status) = self.try_wait()? {
                        break Ok(TimeoutRet::Exited(status));
                    } // otherwise waits again
                }
                Ok(Some(Signal::SIGTERM)) if ignore_term => {} // waits again
                // SAFETY: nix's Signal is also a valid rustix Signal.
                Ok(Some(signal)) => {
                    break Ok(TimeoutRet::Interrupted(unsafe {
                        RixSignal::from_raw_unchecked(signal as _)
                    }));
                }
                Err(e) => break Err(e),
            }
            remaining = timeout.saturating_sub(start.elapsed());
        }
    }
}

/// These signals must be blocked before calling [`ChildExt::wait_or_timeout`].
/// Consider unblocking them in the child's pre-exec hook.
pub fn timeout_signal_set() -> SigSet {
    let mut set = SigSet::empty();
    set.add(Signal::SIGALRM);
    set.add(Signal::SIGINT);
    set.add(Signal::SIGQUIT);
    set.add(Signal::SIGHUP);
    set.add(Signal::SIGTERM);
    set.add(Signal::SIGPIPE);
    set.add(Signal::SIGUSR1);
    set.add(Signal::SIGUSR2);
    set.add(Signal::SIGCHLD);
    set
}

/// Unblocks a signal from the current thread.
pub fn unblock_signal(signal: RixSignal) -> io::Result<()> {
    let mut set = SigSet::empty();
    // SAFETY: rustix's Signal is also a valid nix Signal.
    set.add(unsafe { Signal::try_from(signal.as_raw()).unwrap_unchecked() });

    set.thread_unblock().map_err(Into::into)
}

/// Ensures there is no overflow on time_t operations. Some BSDs (notably XNU)
/// will return EINVAL otherwise; POSIX only defines it up to 10e8, so we cap
/// it on all targets we do not trust to support the full integer range.
const MAX_KTIME_T: Duration = if cfg!(target_os = "linux") {
    Duration::from_secs(9_223_372_036)
} else {
    Duration::from_secs(100_000_000)
};

/// Sets up a timer on SIGALRM for platforms that support POSIX.1-2008 realtime
/// clock extensions. Notably, both Android and Redox do not support the latter
/// fallback since it was removed in that same spec.
#[cfg(not(any(target_vendor = "apple", target_os = "openbsd", target_os = "windows")))]
mod timer {
    use super::MAX_KTIME_T;
    use std::io;
    use std::ptr::null_mut;
    use std::time::Duration;
    #[cfg(any(target_os = "redox", target_os = "android"))]
    use timer_sys as libc; // Complements their libc bindings.

    pub(super) struct Timer(libc::timer_t);

    impl Timer {
        pub(super) fn new() -> io::Result<Self> {
            use std::mem::MaybeUninit;

            // SAFETY: we must zero the reserved, private bits and other fields.
            // We cannot use nix or rustix because they don't support it in Redox.
            let mut sev: libc::sigevent = unsafe { MaybeUninit::zeroed().assume_init() };
            sev.sigev_notify = libc::SIGEV_SIGNAL;
            sev.sigev_signo = libc::SIGALRM;

            // SAFETY: On cygwin, it's a u64; otherwise, a ptr with exposed provenance.
            let mut timer_id = unsafe { MaybeUninit::zeroed().assume_init() };
            // SAFETY: All values are properly initialized.
            if unsafe { libc::timer_create(libc::CLOCK_MONOTONIC, &raw mut sev, &raw mut timer_id) }
                == -1
            {
                return Err(io::Error::last_os_error());
            }

            Ok(Self(timer_id))
        }

        pub(super) fn arm(&mut self, timeout: Duration) -> Result<(), io::Error> {
            let timeout = timeout.min(MAX_KTIME_T).max(Duration::from_micros(1));
            let time = libc::itimerspec {
                it_interval: libc::timespec {
                    tv_sec: 0,
                    tv_nsec: 0,
                },
                it_value: libc::timespec {
                    tv_sec: timeout.as_secs() as _,
                    tv_nsec: timeout.subsec_nanos() as _,
                },
            };

            // SAFETY: All values are properly initialized.
            if unsafe { libc::timer_settime(self.0, 0, &raw const time, null_mut()) } == -1 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        }
    }

    impl Drop for Timer {
        fn drop(&mut self) {
            unsafe { libc::timer_delete(self.0) };
        }
    }

    /// Complements the libc bindings of Redox and Android with missing items.
    #[cfg(any(target_os = "redox", target_os = "android"))]
    #[allow(non_camel_case_types)]
    mod timer_sys {
        pub(super) use libc::{CLOCK_MONOTONIC, SIGALRM, timespec};
        #[cfg(not(target_os = "redox"))]
        pub(super) use libc::{SIGEV_SIGNAL, sigevent};

        pub(super) type timer_t = *mut libc::c_void;

        unsafe extern "C" {
            pub(super) fn timer_settime(
                timerid: timer_t,
                flags: libc::c_int,
                new_value: *const itimerspec,
                old_value: *mut itimerspec,
            ) -> libc::c_int;

            pub(super) fn timer_create(
                clockid: libc::clockid_t,
                sevp: *mut sigevent,
                timerid: *mut timer_t,
            ) -> libc::c_int;

            pub(super) fn timer_delete(timerid: timer_t) -> libc::c_int;
        }

        #[repr(C)]
        #[derive(Clone, Copy, Debug)]
        pub(super) struct itimerspec {
            pub(super) it_interval: timespec,
            pub(super) it_value: timespec,
        }

        #[cfg(target_os = "redox")]
        pub(super) const SIGEV_SIGNAL: libc::c_int = 0;

        #[repr(C)]
        #[derive(Clone, Copy, Debug)]
        #[cfg(target_os = "redox")]
        pub(super) struct sigevent {
            pub(super) sigev_value: libc::sigval,
            pub(super) sigev_signo: libc::c_int,
            pub(super) sigev_notify: libc::c_int,
            pub(super) sigev_notify_thread_id: libc::c_int,
            #[cfg(target_pointer_width = "64")]
            __unused1: std::mem::MaybeUninit<[libc::c_int; 11]>,
            #[cfg(target_pointer_width = "32")]
            __unused1: std::mem::MaybeUninit<[libc::c_int; 12]>,
        }
    }
}

/// Sets up a timer on SIGALRM for platforms that do not support POSIX.1-2008
/// realtime clock extensions. Notably, Darwin, OpenBSD, and Windows.
#[cfg(any(target_vendor = "apple", target_os = "openbsd", target_os = "windows"))]
mod timer {
    use super::MAX_KTIME_T;
    use nix::errno::Errno;
    use std::io;
    use std::ptr::null_mut;
    use std::time::Duration;

    pub(super) struct Timer;

    impl Timer {
        #[allow(clippy::unnecessary_wraps)]
        pub(super) fn new() -> io::Result<Self> {
            Ok(Self)
        }

        pub(super) fn arm(&mut self, timeout: Duration) -> io::Result<()> {
            let timeout = timeout.min(MAX_KTIME_T).max(Duration::from_micros(1));
            let time = libc::itimerval {
                it_interval: libc::timeval {
                    tv_sec: 0,
                    tv_usec: 0,
                },
                it_value: libc::timeval {
                    tv_sec: timeout.as_secs() as _,
                    tv_usec: timeout.subsec_micros() as _,
                },
            };

            // SAFETY: All values are properly initialized.
            Errno::result(unsafe {
                libc::setitimer(libc::ITIMER_REAL, &raw const time, null_mut())
            })?;
            Ok(())
        }
    }
}

impl Timer {
    fn timed_sigwait(&mut self, timeout: Duration) -> io::Result<Option<Signal>> {
        self.arm(timeout)?;

        let set = timeout_signal_set();
        let mut sig = 0;
        // SAFETY: All values are properly initialized.
        let res = unsafe { libc::sigwait(set.as_ref(), &raw mut sig) };

        if res != 0 {
            Err(io::Error::from_raw_os_error(res))
        } else if sig == libc::SIGALRM {
            Ok(None)
        } else {
            Ok(Some(Signal::try_from(sig)?))
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(not(target_os = "redox"))]
    fn test_getsid() {
        use super::{getpid, getsid};

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
