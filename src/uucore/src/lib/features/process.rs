// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (vars) cvar exitstatus cmdline kworker getsid getpid
// spell-checker:ignore (sys/unix) WIFSIGNALED ESRCH
// spell-checker:ignore pgrep pwait snice getpgrp SRCH

use libc::{gid_t, pid_t, uid_t};
use rustix::process::{
    Pid, Signal, kill_current_process_group, kill_process, test_kill_current_process_group,
    test_kill_process,
};
use std::io;
use std::process::Child;
use std::process::ExitStatus;
use std::sync::atomic;
use std::sync::atomic::AtomicBool;
use std::thread;
use std::time::{Duration, Instant};

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
        // Restore the previous disposition.
        unsafe { libc::sigaction(sig_raw, &raw const old, std::ptr::null_mut()) };
        res.map_err(io::Error::from)
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
