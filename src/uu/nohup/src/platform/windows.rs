// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::fs::OpenOptions;
use std::io::{Error, IsTerminal as _};
use std::os::windows::process::CommandExt as _;
use std::process::{Command, Stdio};
use uucore::error::UResult;
use windows_sys::Win32::System::Threading::DETACHED_PROCESS;

use crate::find_stdout;

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
    if std::io::stdin().is_terminal() {
        command.stdin(Stdio::null());
    }
    if std::io::stdout().is_terminal() {
        command.stdout(find_stdout()?);
    }
    if std::io::stderr().is_terminal() {
        command.stderr(Stdio::inherit());
    }

    match command.creation_flags(DETACHED_PROCESS).spawn() {
        Ok(_) => Ok(None),
        Err(e) => Ok(Some(e)),
    }
}

/// Windows has no file mode to set on the output file.
pub(crate) fn set_output_file_mode(_opt: &mut OpenOptions) {}
