// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::fs::OpenOptions;
use std::io::{self, Error, IsTerminal as _};
use std::os::windows::io::{AsHandle as _, OwnedHandle};
use std::os::windows::process::CommandExt as _;
use std::process::{Command, Stdio};
use thiserror::Error as ThisError;
use uucore::error::{UError, UResult};
use uucore::translate;
use windows_sys::Win32::System::Threading::DETACHED_PROCESS;

use crate::find_stdout;

#[derive(Debug, ThisError)]
enum PlatformError {
    #[error("{}", translate!("nohup-error-cannot-replace", "name" => (*_0), "err" => _1))]
    CannotReplace(&'static str, #[source] Error),
}

impl UError for PlatformError {
    fn code(&self) -> i32 {
        2
    }
}

/// Nothing to do before spawning: Windows has no `SIGHUP`, and the standard
/// streams are redirected on the child in [`run`] instead of on this process.
#[allow(
    clippy::unnecessary_wraps,
    reason = "signature shared with the Unix implementation, which can fail"
)]
pub(crate) fn prepare() -> UResult<()> {
    Ok(())
}

/// Spawn `command` detached from the console.
///
/// Returns `None` once the child is running, or the error that made the spawn
/// fail.
pub(crate) fn run(command: &mut Command) -> UResult<Option<Error>> {
    if io::stdin().is_terminal() {
        command.stdin(Stdio::null());
    }

    let stderr_is_terminal = io::stderr().is_terminal();

    if io::stdout().is_terminal() {
        // `find_stdout` appends to `nohup.out` and reports it on stderr, so it
        // must only be called once: give the child a second handle to the very
        // same file instead of opening it again.
        let nohup_out = find_stdout()?;
        if stderr_is_terminal {
            let dup = nohup_out
                .try_clone()
                .map_err(|e| PlatformError::CannotReplace("STDERR", e))?;
            command.stderr(dup);
        }
        command.stdout(nohup_out);
    } else if stderr_is_terminal {
        // stdout is already redirected, so follow it like the Unix side does
        // with `dup2_stderr(stdout())` rather than leaving stderr on the
        // console, which a `DETACHED_PROCESS` child no longer has.
        let dup: OwnedHandle = io::stdout()
            .as_handle()
            .try_clone_to_owned()
            .map_err(|e| PlatformError::CannotReplace("STDERR", e))?;
        command.stderr(Stdio::from(dup));
    }

    match command.creation_flags(DETACHED_PROCESS).spawn() {
        Ok(_) => Ok(None),
        Err(e) => Ok(Some(e)),
    }
}

/// Windows has no file mode to set on the output file.
pub(crate) fn set_output_file_mode(_opt: &mut OpenOptions) {}
