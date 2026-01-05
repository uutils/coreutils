// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (vars) cvar exitstatus cmdline kworker getsid getpid
// spell-checker:ignore (sys/unix) WIFSIGNALED ESRCH
// spell-checker:ignore pgrep pwait snice getpgrp

#[cfg(feature = "pipes")]
use std::marker::PhantomData;

#[cfg(feature = "pipes")]
use crate::pipes::pipe;
#[cfg(feature = "pipes")]
use ::{
    nix::sys::select::FdSet,
    nix::sys::select::select,
    nix::sys::signal::{self, signal},
    nix::sys::time::TimeVal,
    std::fs::File,
    std::io::{Read, Write},
    std::os::fd::AsFd,
    std::process::Command,
    std::process::ExitStatus,
    std::sync::Mutex,
    std::time::Duration,
    std::time::Instant,
};
use libc::{c_int, gid_t, pid_t, uid_t};
#[cfg(not(target_os = "redox"))]
use nix::errno::Errno;
use nix::sys::signal::Signal;
use std::{io, process::Child};

/// Not all platforms support uncapped times (read: macOS). However,
/// we will conform to POSIX for portability.
/// <https://pubs.opengroup.org/onlinepubs/007904875/basedefs/sys/types.h.html#tag_13_67>
#[cfg(feature = "pipes")]
const TIME_T_POSIX_MAX: u64 = 100_000_000;

/// Not all platforms support uncapped times (read: macOS). However,
/// we will conform to POSIX for portability.
/// <https://pubs.opengroup.org/onlinepubs/007904875/basedefs/sys/types.h.html#tag_13_67>
#[cfg(feature = "pipes")]
const SUSECONDS_T_POSIX_MAX: u32 = 1_000_000;

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
    #[cfg(feature = "pipes")]
    fn wait_or_timeout(
        &mut self,
        timeout: Duration,
        self_pipe: &mut SelfPipe,
    ) -> io::Result<WaitOrTimeoutRet>;
}

#[cfg(feature = "pipes")]
pub struct SelfPipe(File, Option<Signal>, PhantomData<*mut ()>);

#[cfg(feature = "pipes")]
pub trait CommandExt {
    fn set_up_timeout(&mut self, other: Option<Signal>) -> io::Result<SelfPipe>;
}

/// Concise enum of [`ChildExt::wait_or_timeout`] possible returns.
#[derive(Debug)]
#[cfg(feature = "pipes")]
pub enum WaitOrTimeoutRet {
    InTime(ExitStatus),
    CustomSignaled,
    TimedOut,
}

impl ChildExt for Child {
    fn send_signal(&mut self, signal: usize) -> io::Result<()> {
        nix::Error::result(unsafe { libc::kill(self.id() as pid_t, signal as i32) })?;
        Ok(())
    }

    fn send_signal_group(&mut self, signal: usize) -> io::Result<()> {
        // Ignore the signal, so we don't go into a signal loop. Some signals will fail
        // the call because they cannot be ignored, but they insta-kill so it's fine.
        if signal != Signal::SIGSTOP as _ && signal != Signal::SIGKILL as _ {
            let err = unsafe { libc::signal(signal as i32, libc::SIG_IGN) } == usize::MAX;
            if err {
                return Err(io::Error::last_os_error());
            }
        }
        nix::Error::result(unsafe { libc::kill(0, signal as i32) })?;
        Ok(())
    }

    #[cfg(feature = "pipes")]
    fn wait_or_timeout(
        &mut self,
        timeout: Duration,
        self_pipe: &mut SelfPipe,
    ) -> io::Result<WaitOrTimeoutRet> {
        // Manually drop stdin
        drop(self.stdin.take());

        let start = Instant::now();
        // This is not a hot loop, it runs exactly once if the process
        // times out, and otherwise will most likely run twice, so that
        // select() ensures we are selecting on the signals we care about.
        // It would only run more than twice if we receive an external
        // signal we are not selecting, select() returns EAGAIN or there is
        // a read error on the pipes (bug on some platforms), but there is no
        // way this creates hot-loop issues anyway.
        loop {
            let mut fd_set = FdSet::new();
            fd_set.insert(self_pipe.0.as_fd());
            let mut timeout_v = duration_to_timeval_elapsed(timeout, start);

            // Perform signal selection.
            match select(None, Some(&mut fd_set), None, None, timeout_v.as_mut())
                .map_err(|x| x as c_int) // Transparent conversion.
            {
                Err(errno::EINTR | errno::EAGAIN) => continue, // Signal interrupted it.
                Err(_) => return Err(io::Error::last_os_error()), // Propagate error.
                Ok(_) => {
                    if start.elapsed() >= timeout && !timeout.is_zero() {
                        return Ok(WaitOrTimeoutRet::TimedOut);
                    }
                    // The set is modified to contain the readable ones;
                    // if empty, we'd stall on the read. However, this may
                    // happen spuriously, so we try to select again.
                    if fd_set.contains(self_pipe.0.as_fd()) {
                        let mut buf = [0];
                        self_pipe.0.read_exact(&mut buf)?;
                        return match buf[0] {
                            // SIGCHLD
                            1 => match self.try_wait()? {
                                Some(e) => Ok(WaitOrTimeoutRet::InTime(e)),
                                None => Ok(WaitOrTimeoutRet::InTime(ExitStatus::default())),
                            },
                            // Received SIGALRM externally, for compat with
                            // GNU timeout we act as if it had timed out.
                            2 => Ok(WaitOrTimeoutRet::TimedOut),
                            // Custom signals on zero timeout still succeed.
                            3 if timeout.is_zero() => {
                                Ok(WaitOrTimeoutRet::InTime(ExitStatus::default()))
                            }
                            // We received a custom signal and fail.
                            3 => Ok(WaitOrTimeoutRet::CustomSignaled),
                            _ => unreachable!(),
                        };
                    }
                }
            }
        }
    }
}

#[cfg(feature = "pipes")]
#[allow(clippy::unnecessary_fallible_conversions, clippy::useless_conversion)]
fn duration_to_timeval_elapsed(time: Duration, start: Instant) -> Option<TimeVal> {
    if time.is_zero() {
        None
    } else {
        let elapsed = start.elapsed();
        // This code ensures we do not overflow on any platform and we keep
        // POSIX conformance. As-casts here are either no-ops or impossible
        // to under/overflow because values are clamped to range or of the
        // same size. If there is underflow, a minimum microsecond is added.
        let seconds = time
            .as_secs()
            .saturating_sub(elapsed.as_secs())
            .clamp(0, TIME_T_POSIX_MAX) as libc::time_t;
        let microseconds = time
            .subsec_micros()
            .saturating_sub(elapsed.subsec_micros())
            .clamp((seconds == 0) as u32, SUSECONDS_T_POSIX_MAX)
            as libc::suseconds_t;

        Some(TimeVal::new(seconds, microseconds))
    }
}

#[cfg(feature = "pipes")]
impl CommandExt for Command {
    fn set_up_timeout(&mut self, other: Option<Signal>) -> io::Result<SelfPipe> {
        static SELF_PIPE_W: Mutex<Option<File>> = Mutex::new(None);
        let (r, w) = pipe()?;
        *SELF_PIPE_W.lock().unwrap() = Some(w);
        extern "C" fn sig_handler(signal: c_int) {
            let mut lock = SELF_PIPE_W.lock();
            let Ok(&mut Some(ref mut writer)) = lock.as_deref_mut() else {
                return;
            };
            if signal == Signal::SIGCHLD as c_int {
                let _ = writer.write(&[1]);
            } else if signal == Signal::SIGALRM as c_int {
                let _ = writer.write(&[2]);
            } else {
                let _ = writer.write(&[3]);
            }
        }
        unsafe {
            signal(Signal::SIGCHLD, signal::SigHandler::Handler(sig_handler))?;
            signal(Signal::SIGALRM, signal::SigHandler::Handler(sig_handler))?;
            if let Some(other) = other {
                signal(other, signal::SigHandler::Handler(sig_handler))?;
            }
        };
        Ok(SelfPipe(r, other, PhantomData))
    }
}

#[cfg(feature = "pipes")]
impl SelfPipe {
    pub fn unset_other(&self) -> io::Result<()> {
        if let Some(other) = self.1 {
            unsafe {
                signal(other, signal::SigHandler::SigDfl)?;
            }
        }
        Ok(())
    }
}

#[cfg(feature = "pipes")]
impl Drop for SelfPipe {
    fn drop(&mut self) {
        let _ = unsafe { signal(Signal::SIGCHLD, signal::SigHandler::SigDfl) };
        let _ = self.unset_other();
    }
}

// The libc/nix crate appear to not have caught up to on Redox's libc, so
// we will just do this manually, which should be fine.
// FIXME: import Errno and try on Redox at some point, then enable them
// throughout uutils. Maybe we could just link to it ourselves, though.
#[cfg(all(not(target_os = "redox"), feature = "pipes"))]
mod errno {
    use super::{Errno, c_int};

    pub const EINTR: c_int = Errno::EINTR as c_int;
    pub const EAGAIN: c_int = Errno::EAGAIN as c_int;
}

#[cfg(all(target_os = "redox", feature = "pipes"))]
mod errno {
    use super::c_int;

    pub const EINTR: c_int = 4;
    pub const EAGAIN: c_int = 11;
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
