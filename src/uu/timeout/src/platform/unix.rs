// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) sigstr setpgid sigchld getpid TTIN TTOU

use std::os::unix::process::CommandExt;
use std::os::unix::process::ExitStatusExt;
use std::process::Child;
use std::sync::atomic;

use rustix::process::{Pid, Signal, getpid, kill_process, setpgid};
use uucore::process::ChildExt;
use uucore::signals::{install_signal_handler, signal_by_name_or_value};

/// Install SIGCHLD handler to ensure waiting for child works even if parent ignored SIGCHLD.
fn install_sigchld() {
    extern "C" fn chld(_: libc::c_int) {}
    let _ = install_signal_handler(Signal::as_raw(Signal::CHILD), chld);
}

/// Install signal handlers for termination signals.
fn install_signal_handlers(term_signal: usize) {
    extern "C" fn handle_signal(sig: libc::c_int) {
        crate::SIGNALED.store(true, atomic::Ordering::Relaxed);
        crate::RECEIVED_SIGNAL.store(sig, atomic::Ordering::Relaxed);
    }

    let sigpipe_ignored = uucore::signals::sigpipe_was_ignored();

    for sig in [
        Signal::ALARM,
        Signal::INT,
        Signal::QUIT,
        Signal::HUP,
        Signal::TERM,
        Signal::PIPE,
        Signal::USR1,
        Signal::USR2,
    ] {
        if sig == Signal::PIPE && sigpipe_ignored {
            continue; // Skip SIGPIPE if it was ignored by parent
        }
        let _ = install_signal_handler(Signal::as_raw(sig), handle_signal);
    }

    if let Some(sig) = signal_from_raw(term_signal as i32) {
        let _ = install_signal_handler(Signal::as_raw(sig), handle_signal);
    }
}

fn signal_from_raw(sig: i32) -> Option<Signal> {
    if sig <= 0 {
        return None;
    }
    // Fast path: standard named signals (SIGHUP, SIGTERM, SIGKILL, etc.)
    if let Some(s) = Signal::from_named_raw(sig) {
        return Some(s);
    }
    // Slow path: realtime signals (SIGRTMIN..=SIGRTMAX).
    #[cfg(target_os = "linux")]
    {
        let rtmin = libc::SIGRTMIN();
        let rtmax = libc::SIGRTMAX();
        if sig >= rtmin && sig <= rtmax {
            return Some(unsafe { Signal::from_raw_unchecked(sig) });
        }
    }

    None
}

/// Configure our own process group, the child's spawn attributes and the
/// signal handlers, right before the child is spawned.
pub(crate) fn prepare(cmd_builder: &mut std::process::Command, foreground: bool, signal: usize) {
    if !foreground {
        let _ = setpgid(Pid::from_raw(0), Pid::from_raw(0));
    }

    {
        #[cfg(target_os = "linux")]
        let death_sig = signal_from_raw(signal as i32);
        let sigpipe_was_ignored = uucore::signals::sigpipe_was_ignored();
        let stdin_was_closed = uucore::signals::stdin_was_closed();

        unsafe {
            cmd_builder.pre_exec(move || {
                // Reset terminal signals to default
                let _ = libc::signal(Signal::as_raw(Signal::TTIN), libc::SIG_DFL);
                let _ = libc::signal(Signal::as_raw(Signal::TTOU), libc::SIG_DFL);
                // Preserve SIGPIPE ignore status if parent had it ignored
                if sigpipe_was_ignored {
                    let _ = libc::signal(Signal::as_raw(Signal::PIPE), libc::SIG_IGN);
                }
                // If stdin was closed before Rust reopened it as /dev/null, close it in child
                if stdin_was_closed {
                    libc::close(libc::STDIN_FILENO);
                }
                #[cfg(target_os = "linux")]
                let _ = rustix::process::set_parent_process_death_signal(death_sig);
                Ok(())
            });
        }
    }

    install_sigchld();
    install_signal_handlers(signal);
}

/// Unix keeps no per-spawn platform state; the type exists so the facade
/// signatures match the Windows implementation (which carries a job object).
pub(crate) struct SpawnState;

/// Nothing to do after spawning on unix.
pub(crate) fn post_spawn(_child: &Child, _foreground: bool) -> SpawnState {
    SpawnState
}

pub(crate) fn send_signal(
    process: &mut Child,
    signal: usize,
    foreground: bool,
    _external: Option<usize>,
    _state: &SpawnState,
) {
    // NOTE: GNU timeout doesn't check for errors of signal.
    // The subprocess might have exited just after the timeout.
    let _ = process.send_signal(signal);
    if signal == 0 || foreground {
        return;
    }
    let _ = process.send_signal_group(signal);
    let kill_signal = signal_by_name_or_value("KILL").unwrap();
    let continued_signal = signal_by_name_or_value("CONT").unwrap();
    if signal != kill_signal && signal != continued_signal {
        let _ = process.send_signal(continued_signal);
        let _ = process.send_signal_group(continued_signal);
    }
}

/// The termination signal received by timeout itself while waiting, if any.
/// Consuming: a second call returns `None`, so read it once per wake.
pub(crate) fn external_signal() -> Option<usize> {
    let received_sig = crate::RECEIVED_SIGNAL.swap(0, atomic::Ordering::Relaxed);
    (received_sig > 0 && received_sig != libc::SIGALRM).then_some(received_sig as usize)
}

/// The signal the child was terminated by, if it was terminated by a signal.
pub(crate) fn status_signal(status: std::process::ExitStatus) -> Option<i32> {
    status.signal()
}

pub(crate) fn preserve_signal_info(signal: libc::c_int) -> libc::c_int {
    // This is needed because timeout is expected to preserve the exit
    // status of its child. It is not the case that utilities have a
    // single simple exit code, that's an illusion some shells
    // provide.  Instead exit status is really two numbers:
    //
    //  - An exit code if the program ran to completion
    //
    //  - A signal number if the program was terminated by a signal
    //
    // The easiest way to preserve the latter seems to be to kill
    // ourselves with whatever signal our child exited with, which is
    // what the following is intended to accomplish.
    if let Some(sig) = signal_from_raw(signal) {
        let _ = kill_process(getpid(), sig);
    }
    signal
}
