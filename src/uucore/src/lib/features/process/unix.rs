// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore sigwait KTIME timeval itimerval setitimer itimer timerid
// spell-checker:ignore sigevent sigev sigval itimerspec signo clockid sevp
// spell-checker:ignore sigset sigaddset sigemptyset

#[cfg(not(any(target_os = "fuchsia", target_os = "haiku")))]
use nix::sys::signal::{self, SigHandler};
use nix::sys::signal::{SigSet, Signal};
use rustix::process::Signal as RixSignal;
use std::io;
#[cfg(not(any(target_os = "fuchsia", target_os = "haiku")))]
use std::process::Child;
#[cfg(not(any(target_os = "fuchsia", target_os = "haiku")))]
use std::time::{Duration, Instant};
#[cfg(not(any(target_os = "fuchsia", target_os = "haiku")))]
use timer::Timer;

#[cfg(not(any(target_os = "fuchsia", target_os = "haiku")))]
use super::{ChildExt, TimeoutRet};

#[cfg(not(any(target_os = "fuchsia", target_os = "haiku")))]
fn signal_from_raw(signal: libc::c_int) -> Option<RixSignal> {
    if let Some(signal) = RixSignal::from_named_raw(signal) {
        return Some(signal);
    }
    #[cfg(any(target_os = "linux", target_os = "android"))]
    if (libc::SIGRTMIN()..=libc::SIGRTMAX()).contains(&signal) {
        // SAFETY: the signal is within the platform's real-time signal range.
        return Some(unsafe { RixSignal::from_raw_unchecked(signal) });
    }
    None
}

#[cfg(not(any(target_os = "fuchsia", target_os = "haiku")))]
impl ChildExt for Child {
    fn send_signal(&mut self, signal: usize) -> io::Result<()> {
        let signal = libc::c_int::try_from(signal)
            .map_err(|_| io::Error::from_raw_os_error(libc::EINVAL))?;
        let pid = rustix::process::Pid::from_child(self);
        if signal == 0 {
            return rustix::process::test_kill_process(pid).map_err(Into::into);
        }
        let signal =
            signal_from_raw(signal).ok_or_else(|| io::Error::from_raw_os_error(libc::EINVAL))?;
        rustix::process::kill_process(pid, signal).map_err(Into::into)
    }

    fn send_signal_group(&mut self, signal: usize) -> io::Result<()> {
        // Send signal to our process group (group 0 = caller's group).
        // This matches GNU coreutils behavior: if the child has remained in our
        // process group, it will receive this signal along with all other processes
        // in the group. If the child has created its own process group (via setpgid),
        // it won't receive this group signal, but will have received the direct signal.

        let signal = libc::c_int::try_from(signal)
            .map_err(|_| io::Error::from_raw_os_error(libc::EINVAL))?;

        // Signal 0 is special - it just checks if process exists, doesn't send anything.
        // No need to manipulate signal handlers for it.
        if signal == 0 {
            return rustix::process::test_kill_process_group(rustix::process::getpgrp())
                .map_err(Into::into);
        }

        let rustix_signal =
            signal_from_raw(signal).ok_or_else(|| io::Error::from_raw_os_error(libc::EINVAL))?;
        if let Ok(signal) = Signal::try_from(signal) {
            // SAFETY: ignoring a signal does not install a callback.
            let old_handler = unsafe { signal::signal(signal, SigHandler::SigIgn) }?;
            let result =
                rustix::process::kill_process_group(rustix::process::getpgrp(), rustix_signal)
                    .map_err(Into::into);
            // SAFETY: restore the handler returned by the successful call above.
            let _ = unsafe { signal::signal(signal, old_handler) };
            return result;
        }

        // Ignore the signal temporarily so we don't receive it ourselves.
        // nix cannot represent real-time signals in its Signal enum.
        // SAFETY: SIG_IGN is a valid handler; signal validates the raw number.
        let old_handler = unsafe { libc::signal(signal, libc::SIG_IGN) };
        if old_handler == libc::SIG_ERR {
            return Err(io::Error::last_os_error());
        }
        let result = rustix::process::kill_process_group(rustix::process::getpgrp(), rustix_signal)
            .map_err(Into::into);
        // Restore the old handler
        // SAFETY: old_handler was returned by a successful signal call.
        let _ = unsafe { libc::signal(signal, old_handler) };
        result
    }

    fn wait_or_timeout(&mut self, timeout: Duration, ignore_term: bool) -> io::Result<TimeoutRet> {
        // .try_wait() doesn't drop stdin, so we do it manually
        drop(self.stdin.take());

        // Waits continuously whenever we receive an external SIGCHLD or
        // we SIGTERM when we are ignoring them.
        let start = Instant::now();
        // Zero disables the timer, but signals must still be handled.
        let mut remaining = (!timeout.is_zero()).then_some(timeout);
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
                Ok(Some(signal)) => break Ok(TimeoutRet::Interrupted(signal as usize)),
                Err(e) => break Err(e),
            }
            if remaining.is_some() {
                remaining = Some(timeout.saturating_sub(start.elapsed()));
            }
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
    let mut set = *SigSet::empty().as_ref();
    // SAFETY: set is initialized and rustix supplies a valid signal number.
    // In particular, a real-time signal need not have a nix Signal variant.
    if unsafe { libc::sigaddset(&raw mut set, signal.as_raw()) } == -1 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: the set was initialized by sigemptyset and changed by sigaddset.
    unsafe { SigSet::from_sigset_t_unchecked(set) }
        .thread_unblock()
        .map_err(Into::into)
}

/// Ensures there is no overflow on `time_t` operations. Some BSDs (notably XNU)
/// will return EINVAL otherwise; POSIX only defines it up to 10e8, so we cap
/// it on all targets we do not trust to support the full integer range.
#[cfg(not(any(target_os = "fuchsia", target_os = "haiku")))]
const MAX_KTIME_T: Duration = if cfg!(target_os = "linux") {
    Duration::from_secs(9_223_372_036)
} else {
    Duration::from_secs(100_000_000)
};

/// Sets up a timer on SIGALRM for platforms that support POSIX.1-2008 realtime
/// clock extensions. Notably, both Android and Redox do not support the latter
/// fallback since it was removed in that same spec.
#[cfg(not(any(
    target_vendor = "apple",
    target_os = "dragonfly",
    target_os = "fuchsia",
    target_os = "haiku",
    target_os = "openbsd",
    windows
)))]
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

            // We must zero the reserved, private bits and other fields.
            // We cannot use nix or rustix because they don't support it in Redox.
            let mut sev = MaybeUninit::<libc::sigevent>::zeroed();

            unsafe {
                (*sev.as_mut_ptr()).sigev_notify = libc::SIGEV_SIGNAL;
                (*sev.as_mut_ptr()).sigev_signo = libc::SIGALRM;
            }

            // On cygwin, it's a u64; otherwise, a ptr with exposed provenance.
            let mut timer_id = MaybeUninit::zeroed();
            // SAFETY: All values are properly initialized.
            if unsafe {
                libc::timer_create(
                    libc::CLOCK_MONOTONIC,
                    sev.as_mut_ptr(),
                    timer_id.as_mut_ptr(),
                )
            } == -1
            {
                return Err(io::Error::last_os_error());
            }

            // SAFETY: `timer_create` returned success and initialized timer_id.
            Ok(Self(unsafe { timer_id.assume_init() }))
        }

        pub(super) fn arm(&mut self, timeout: Duration) -> Result<(), io::Error> {
            let timeout = timeout.min(MAX_KTIME_T).max(Duration::from_micros(1));
            // `timespec` has private padding members on time64 targets, so its
            // fields cannot be listed in a struct literal; start from the
            // zeroed default and fill in the ones we care about.
            let mut time = libc::itimerspec {
                it_interval: libc::timespec::default(),
                it_value: libc::timespec::default(),
            };
            time.it_value.tv_sec = timeout.as_secs() as _;
            time.it_value.tv_nsec = timeout.subsec_nanos() as _;

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
/// realtime clock extensions.
#[cfg(any(
    target_vendor = "apple",
    target_os = "dragonfly",
    target_os = "openbsd",
    windows
))]
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

#[cfg(not(any(target_os = "fuchsia", target_os = "haiku")))]
impl Timer {
    fn timed_sigwait(&mut self, timeout: Option<Duration>) -> io::Result<Option<Signal>> {
        if let Some(timeout) = timeout {
            self.arm(timeout)?;
        }

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
