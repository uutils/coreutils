// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Windows implementation of `kill`'s platform facade, built on the signal
//! emulation in [`uucore::process`]. STOP (no process-suspend API) and
//! process groups (`pid <= 0`) have no Windows emulation and are rejected.

use std::io;

use uucore::process::send_signal_to_pid;
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
        Ok(pid) if pid != 0 => send_signal_to_pid(pid, sig),
        _ => Err(unsupported(translate!(
            "kill-error-process-groups-unsupported"
        ))),
    }
}
