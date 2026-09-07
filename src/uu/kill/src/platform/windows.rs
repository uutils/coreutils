// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore pids

//! Windows implementation of `kill`'s platform facade, built on the signal
//! emulation in [`uucore::process`]. PID 0 targets the Job object `kill` runs
//! in, the closest Windows analog of a process group. STOP (no process-suspend
//! API) and negative pids (no way to name another process's group) are
//! rejected.

use std::io;

use uucore::process::{enable_debug_privilege, send_signal_to_own_group, send_signal_to_pid};
use uucore::translate;

const SIGNAL_STOP: usize = 19;

fn unsupported(message: String) -> io::Error {
    io::Error::new(io::ErrorKind::Unsupported, message)
}

pub(crate) fn send_signal(pid: i32, sig: usize) -> io::Result<()> {
    if sig == SIGNAL_STOP {
        return Err(unsupported(translate!("kill-error-unsupported-signal")));
    }
    match u32::try_from(pid) {
        // Fails exactly for pid < 0.
        Err(_) => Err(unsupported(translate!(
            "kill-error-negative-pid-unsupported"
        ))),
        Ok(pid) => {
            // Group members need the same rights as a single target.
            enable_debug_privilege();
            if pid == 0 {
                send_signal_to_own_group(sig)
            } else {
                send_signal_to_pid(pid, sig)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::send_signal;

    #[test]
    fn negative_pids_are_rejected_without_touching_any_process() {
        for pid in [-1, -2, i32::MIN] {
            assert_eq!(
                send_signal(pid, 9).unwrap_err().kind(),
                std::io::ErrorKind::Unsupported
            );
        }
    }

    #[test]
    fn stop_is_rejected_for_every_pid_including_zero() {
        assert_eq!(
            send_signal(0, 19).unwrap_err().kind(),
            std::io::ErrorKind::Unsupported
        );
    }
}
