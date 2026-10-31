// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore pids ESRCH

use std::cmp::Ordering;
use std::io;

use rustix::process::{
    Pid, Signal, kill_current_process_group, kill_process, kill_process_group,
    test_kill_current_process_group, test_kill_process, test_kill_process_group,
};

// rustix's `Signal` rejects libc-reserved realtime signals, so fall back to a
// raw `libc::kill` for any value its safe constructor doesn't recognize.
fn raw_kill(pid: i32, sig: usize) -> io::Result<()> {
    let sig = i32::try_from(sig).map_err(|_| io::Error::from_raw_os_error(libc::EINVAL))?;
    // SAFETY: plain FFI call; `kill` has no memory-safety preconditions.
    if unsafe { libc::kill(pid as libc::pid_t, sig) } == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

/// Deliver `sig` to `pid` with kill(2) semantics: positive pids target one
/// process, 0 the current process group, negative pids the group `-pid`.
pub(crate) fn send_signal(pid: i32, sig: usize) -> io::Result<()> {
    // Standard named signals use rustix's typed API; anything its safe
    // constructor doesn't recognize (realtime/reserved) falls back to libc.
    let named = (sig != 0)
        .then(|| i32::try_from(sig).ok().and_then(Signal::from_named_raw))
        .flatten();
    match pid.cmp(&0) {
        Ordering::Equal => match named {
            _ if sig == 0 => test_kill_current_process_group().map_err(io::Error::from),
            Some(s) => kill_current_process_group(s).map_err(io::Error::from),
            None => raw_kill(0, sig),
        },
        Ordering::Greater => {
            let pid = Pid::from_raw(pid).expect("pid > 0 guaranteed by Ordering::Greater");
            match named {
                _ if sig == 0 => test_kill_process(pid).map_err(io::Error::from),
                Some(s) => kill_process(pid, s).map_err(io::Error::from),
                None => raw_kill(pid.as_raw_nonzero().get(), sig),
            }
        }
        Ordering::Less => {
            // i32::MIN cannot be negated, so no such process group can exist.
            let Some(abs_pid) = pid.checked_neg() else {
                return Err(io::Error::from_raw_os_error(libc::ESRCH));
            };
            let pid =
                Pid::from_raw(abs_pid).expect("abs_pid > 0 since pid < 0 and pid != i32::MIN");
            match named {
                _ if sig == 0 => test_kill_process_group(pid).map_err(io::Error::from),
                Some(s) => kill_process_group(pid, s).map_err(io::Error::from),
                None => raw_kill(-pid.as_raw_nonzero().get(), sig),
            }
        }
    }
}
