// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Windows implementation of `timeout`'s platform facade, built on the signal
//! emulation in [`uucore::process`] (which maps POSIX signal numbers to
//! Windows primitives). Non-foreground mode spawns the child in a new console
//! process group tracked by a Job Object, so a lethal signal kills the whole
//! tree and `INT`/`QUIT` arrive as a catchable `CTRL_BREAK_EVENT`.

use std::process::Child;
use std::sync::OnceLock;

use uucore::process::{
    Job, configure_process_group, enable_ctrl_forwarding, last_ctrl_signal,
    send_signal_to_console_group, send_signal_to_process, send_signal_to_tree,
};

const SIGNAL_INT: usize = 2;
const SIGNAL_QUIT: usize = 3;

/// The job object tracking the child's process tree (non-foreground mode
/// only; `None` also when job assignment failed and we degraded to
/// per-process signalling).
static JOB: OnceLock<Job> = OnceLock::new();

/// Configure the child's spawn attributes and console-event forwarding, right
/// before the child is spawned.
pub(crate) fn prepare(cmd_builder: &mut std::process::Command, foreground: bool, _signal: usize) {
    // Record Ctrl-C/Ctrl-Break/console-close as forwardable signal numbers
    // instead of dying to them (the unix code installs signal handlers here).
    let _ = enable_ctrl_forwarding();

    if !foreground {
        // The analog of unix setpgid(0, 0): the child leads its own console
        // group, so the console's Ctrl-C no longer reaches it directly and
        // CTRL_BREAK can target exactly its group.
        configure_process_group(cmd_builder);
    }
}

/// Put the freshly spawned child in a job so a lethal signal can terminate
/// its entire process tree. Degrades silently to per-process signalling when
/// jobs are unavailable (mirroring the ignored `setpgid` result on unix).
///
/// A child that spawns a grandchild before the assignment completes escapes
/// the job; the window is sub-millisecond and unix has the analogous escape
/// (a child can leave the process group via setpgid).
pub(crate) fn post_spawn(child: &Child, foreground: bool) {
    if foreground {
        return;
    }
    if let Ok(job) = Job::new() {
        if job.assign(child).is_ok() {
            let _ = JOB.set(job);
        }
    }
}

pub(crate) fn send_signal(process: &mut Child, signal: usize, foreground: bool) {
    // GNU timeout ignores signal-send errors (the child may have just exited),
    // hence the discarded results below.
    if foreground {
        if matches!(signal, SIGNAL_INT | SIGNAL_QUIT) && last_ctrl_signal() == Some(signal) {
            // A foreground child shares our console group: the console already
            // delivered this externally received event to it, and re-sending
            // cannot be targeted without hitting ourselves.
            return;
        }
        let _ = send_signal_to_process(process, signal);
    } else if matches!(signal, SIGNAL_INT | SIGNAL_QUIT) {
        // Deliverable as a catchable CTRL_BREAK to the child's group; without
        // a console, fall back to termination so the signal is never lost.
        if send_signal_to_console_group(process.id(), signal).is_err() {
            let _ = send_signal_to_tree(process, JOB.get(), signal);
        }
    } else {
        let _ = send_signal_to_tree(process, JOB.get(), signal);
    }
}

/// The signal number of the console event received by timeout itself while
/// waiting, if any.
pub(crate) fn external_signal() -> Option<usize> {
    last_ctrl_signal()
}

/// Windows exit statuses carry no signal information: terminated children
/// report their forced `128 + signal` value through `status.code()` instead.
pub(crate) fn status_signal(_status: std::process::ExitStatus) -> Option<i32> {
    None
}

/// No two-channel exit status exists on Windows; the exit code already
/// carries the signal information (`128 + signal`), so there is nothing to
/// preserve by re-signalling ourselves like the unix implementation does.
pub(crate) fn preserve_signal_info(signal: i32) -> i32 {
    signal
}
