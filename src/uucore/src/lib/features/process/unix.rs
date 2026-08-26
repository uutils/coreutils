// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (vars) cvar exitstatus cmdline kworker getsid getpid
// spell-checker:ignore (sys/unix) WIFSIGNALED ESRCH
// spell-checker:ignore pgrep pwait snice getpgrp SRCH sigset sigemptyset sigaddset sigpending sigismember sigfillset pthread sigmask
// spell-checker:ignore sigwait KTIME timeval itimerval setitimer itimer timerid
// spell-checker:ignore sigevent sigev sigval itimerspec signo clockid sevp

use libc::{gid_t, pid_t, uid_t};
use rustix::process::{
    Pid, Signal, kill_current_process_group, kill_process, test_kill_current_process_group,
    test_kill_process,
};
use std::io;
use std::process::Child;
use std::time::{Duration, Instant};
use timer::Timer;

use super::{ChildExt, TimeoutRet};

/// `geteuid()` returns the effective user ID of the calling process.
pub fn geteuid() -> uid_t {
    rustix::process::geteuid().as_raw()
}

/// `getpgrp()` returns the process group ID of the calling process.
pub fn getpgrp() -> pid_t {
    rustix::process::getpgrp().as_raw_pid()
}

/// `getegid()` returns the effective group ID of the calling process.
pub fn getegid() -> gid_t {
    rustix::process::getegid().as_raw()
}

/// `getgid()` returns the real group ID of the calling process.
pub fn getgid() -> gid_t {
    rustix::process::getgid().as_raw()
}

/// `getuid()` returns the real user ID of the calling process.
pub fn getuid() -> uid_t {
    rustix::process::getuid().as_raw()
}

/// `getpid()` returns the pid of the calling process.
pub fn getpid() -> pid_t {
    rustix::process::getpid().as_raw_pid()
}

/// `getsid()` returns the session ID of the process with process ID pid.
///
/// If pid is 0, getsid() returns the session ID of the calling process.
///
/// # Error
///
/// - `EPERM` A process with process ID pid exists, but it is not in the same session as the calling process, and the implementation considers this an error.
/// - `ESRCH` No process with process ID pid was found.
///
///
/// # Platform
///
/// This function only support standard POSIX implementation platform,
/// so some system such as redox doesn't supported.
#[cfg(not(target_os = "redox"))]
pub fn getsid(pid: i32) -> Result<pid_t, rustix::io::Errno> {
    let pid = match pid {
        0 => None,
        _ => Some(Pid::from_raw(pid).ok_or(rustix::io::Errno::SRCH)?),
    };
    rustix::process::getsid(pid).map(Pid::as_raw_pid)
}

/// Build a rustix [`Signal`] from a raw number, including real-time signals
/// (`SIGRTMIN..=SIGRTMAX`). Real-time signals are not "named", so
/// [`Signal::from_named_raw`] rejects them and we build them from the raw value.
///
/// Validation (named signals plus the real-time range) is shared with `env` via
/// [`crate::signals::signal_from_raw`] when the `signals` feature is enabled —
/// which the signal-sending callers (`kill`, `timeout`) always do. The
/// `process`-only utilities (`id`, `whoami`, …) never send signals, so they fall
/// back to a named-signal-only converter rather than pull in the whole module.
#[cfg(feature = "signals")]
fn signal_from_value(signal: usize) -> io::Result<Signal> {
    let raw = crate::signals::signal_from_raw(signal)
        .ok_or_else(|| io::Error::from_raw_os_error(libc::EINVAL))?;
    // SAFETY: `signal_from_raw` only returns named or real-time signal numbers,
    // both of which are valid `Signal` values on this platform.
    Ok(Signal::from_named_raw(raw).unwrap_or_else(|| unsafe { Signal::from_raw_unchecked(raw) }))
}

#[cfg(not(feature = "signals"))]
fn signal_from_value(signal: usize) -> io::Result<Signal> {
    i32::try_from(signal)
        .ok()
        .filter(|&s| s > 0)
        .and_then(Signal::from_named_raw)
        .ok_or_else(|| io::Error::from_raw_os_error(libc::EINVAL))
}

/// Discard any pending instance of `sig` for this process.
///
/// Only ever reaps a signal that is both blocked and pending, so `sigwait` returns
/// at once instead of blocking.
fn discard_pending(sig: i32) {
    // SAFETY: `pending` and `set` are initialized before use, and `sigwait` is only
    // reached once `sigpending` reports `sig` as pending (hence blocked), so it
    // returns immediately.
    unsafe {
        let mut pending: libc::sigset_t = std::mem::zeroed();
        if libc::sigpending(&raw mut pending) == -1
            || libc::sigismember(&raw const pending, sig) != 1
        {
            return;
        }
        let mut set: libc::sigset_t = std::mem::zeroed();
        libc::sigemptyset(&raw mut set);
        libc::sigaddset(&raw mut set, sig);
        let mut caught = 0;
        libc::sigwait(&raw const set, &raw mut caught);
    }
}

impl ChildExt for Child {
    fn send_signal(&mut self, signal: usize) -> io::Result<()> {
        let pid = Pid::from_raw(self.id() as pid_t)
            .ok_or_else(|| io::Error::from_raw_os_error(libc::EINVAL))?;
        // signal == 0 only probes whether the pid is still alive.
        if signal == 0 {
            return test_kill_process(pid).map_err(io::Error::from);
        }
        kill_process(pid, signal_from_value(signal)?).map_err(io::Error::from)
    }

    fn send_signal_group(&mut self, signal: usize) -> io::Result<()> {
        // Send signal to our process group (group 0 = caller's group).
        // This matches GNU coreutils behavior: if the child has remained in our
        // process group, it will receive this signal along with all other processes
        // in the group. If the child has created its own process group (via setpgid),
        // it won't receive this group signal, but will have received the direct signal.

        // Signal 0 is special - it just checks if the group exists, doesn't send anything.
        // No need to manipulate signal handlers for it.
        if signal == 0 {
            return test_kill_current_process_group().map_err(io::Error::from);
        }

        let sig = signal_from_value(signal)?;
        let sig_raw = sig.as_raw();

        // Ignore the signal temporarily so we don't receive it ourselves. rustix
        // deliberately does not wrap sigaction (see its not_implemented::libc_internals);
        // its only equivalent is the experimental `runtime` module, which is UB in a
        // process that links libc. Signal disposition is left to libc, so use it here.
        // SAFETY: a zeroed sigaction with SIG_IGN is a valid disposition; we restore the
        // previous one right after sending to our own process group.
        let mut ignore: libc::sigaction = unsafe { std::mem::zeroed() };
        ignore.sa_sigaction = libc::SIG_IGN;
        let mut old: libc::sigaction = unsafe { std::mem::zeroed() };
        if unsafe { libc::sigaction(sig_raw, &raw const ignore, &raw mut old) } == -1 {
            return Err(io::Error::last_os_error());
        }
        let res = kill_current_process_group(sig);
        // SIG_IGN alone is not enough when the caller blocks the signal to consume it
        // with `sigwait` (as `timeout` does): the kernel only discards an ignored
        // signal that is *not* blocked, so a blocked one stays queued and the caller
        // would read back the very signal it just sent to its own group. Drop that
        // self-delivered instance.
        discard_pending(sig_raw);
        // Restore the previous disposition.
        unsafe { libc::sigaction(sig_raw, &raw const old, std::ptr::null_mut()) };
        res.map_err(io::Error::from)
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
                Ok(Some(Signal::CHILD)) => {
                    if let Some(status) = self.try_wait()? {
                        break Ok(TimeoutRet::Exited(status));
                    } // otherwise waits again
                }
                Ok(Some(Signal::TERM)) if ignore_term => {} // waits again
                Ok(Some(signal)) => break Ok(TimeoutRet::Interrupted(signal.as_raw() as usize)),
                Err(e) => break Err(e),
            }
            remaining = timeout.saturating_sub(start.elapsed());
        }
    }
}

/// A set of signals, i.e. a `sigset_t`. rustix leaves the signal mask to libc
/// (see its `not_implemented::libc_internals`), so this goes straight to libc.
pub struct SignalSet(libc::sigset_t);

impl SignalSet {
    /// An empty set.
    pub fn empty() -> Self {
        // SAFETY: `sigemptyset` initializes the set.
        let mut set = unsafe { std::mem::zeroed() };
        unsafe { libc::sigemptyset(&raw mut set) };
        Self(set)
    }

    /// Adds `signal` to the set.
    pub fn add(&mut self, signal: Signal) {
        // SAFETY: the set is initialized and `signal` is a valid signal number.
        unsafe { libc::sigaddset(&raw mut self.0, signal.as_raw()) };
    }

    /// Blocks every signal of the set for the calling thread.
    pub fn thread_block(&self) -> io::Result<()> {
        self.mask(libc::SIG_BLOCK)
    }

    /// Unblocks every signal of the set for the calling thread. Async-signal-safe,
    /// so it can be called from a pre-exec hook.
    pub fn thread_unblock(&self) -> io::Result<()> {
        self.mask(libc::SIG_UNBLOCK)
    }

    fn mask(&self, how: libc::c_int) -> io::Result<()> {
        // SAFETY: the set is initialized and `how` is one of the documented values.
        let res = unsafe { libc::pthread_sigmask(how, &raw const self.0, std::ptr::null_mut()) };
        if res == 0 {
            Ok(())
        } else {
            Err(io::Error::from_raw_os_error(res))
        }
    }

    fn as_raw(&self) -> *const libc::sigset_t {
        &raw const self.0
    }
}

/// These signals must be blocked before calling [`ChildExt::wait_or_timeout`].
/// Consider unblocking them in the child's pre-exec hook.
pub fn timeout_signal_set() -> SignalSet {
    let mut set = SignalSet::empty();
    for signal in [
        Signal::ALARM,
        Signal::INT,
        Signal::QUIT,
        Signal::HUP,
        Signal::TERM,
        Signal::PIPE,
        Signal::USR1,
        Signal::USR2,
        Signal::CHILD,
    ] {
        set.add(signal);
    }
    set
}

/// Unblocks a signal from the current thread.
pub fn unblock_signal(signal: Signal) -> io::Result<()> {
    let mut set = SignalSet::empty();
    set.add(signal);
    set.thread_unblock()
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

        pub(super) type timer_t = *mut core::ffi::c_void;

        unsafe extern "C" {
            pub(super) fn timer_settime(
                timerid: timer_t,
                flags: core::ffi::c_int,
                new_value: *const itimerspec,
                old_value: *mut itimerspec,
            ) -> core::ffi::c_int;

            pub(super) fn timer_create(
                clockid: libc::clockid_t,
                sevp: *mut sigevent,
                timerid: *mut timer_t,
            ) -> core::ffi::c_int;

            pub(super) fn timer_delete(timerid: timer_t) -> core::ffi::c_int;
        }

        #[repr(C)]
        #[derive(Clone, Copy, Debug)]
        pub(super) struct itimerspec {
            pub(super) it_interval: timespec,
            pub(super) it_value: timespec,
        }

        #[cfg(target_os = "redox")]
        pub(super) const SIGEV_SIGNAL: core::ffi::c_int = 0;

        #[repr(C)]
        #[derive(Clone, Copy, Debug)]
        #[cfg(target_os = "redox")]
        pub(super) struct sigevent {
            pub(super) sigev_value: libc::sigval,
            pub(super) sigev_signo: core::ffi::c_int,
            pub(super) sigev_notify: core::ffi::c_int,
            pub(super) sigev_notify_thread_id: core::ffi::c_int,
            #[cfg(target_pointer_width = "64")]
            __unused1: std::mem::MaybeUninit<[core::ffi::c_int; 11]>,
            #[cfg(target_pointer_width = "32")]
            __unused1: std::mem::MaybeUninit<[core::ffi::c_int; 12]>,
        }
    }
}

/// Sets up a timer on SIGALRM for platforms that do not support POSIX.1-2008
/// realtime clock extensions. Notably, Darwin, OpenBSD, and Windows.
#[cfg(any(target_vendor = "apple", target_os = "openbsd", target_os = "windows"))]
mod timer {
    use super::MAX_KTIME_T;
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
            if unsafe { libc::setitimer(libc::ITIMER_REAL, &raw const time, null_mut()) } == -1 {
                return Err(io::Error::last_os_error());
            }
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
        let res = unsafe { libc::sigwait(set.as_raw(), &raw mut sig) };

        if res != 0 {
            Err(io::Error::from_raw_os_error(res))
        } else if sig == libc::SIGALRM {
            Ok(None)
        } else {
            // SAFETY: `sigwait` only reports a signal of the set we passed it.
            Ok(Some(unsafe { Signal::from_raw_unchecked(sig) }))
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
