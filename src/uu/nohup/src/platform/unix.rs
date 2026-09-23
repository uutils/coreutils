// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) SIGHUP cproc vprocmgr homeout

use std::fs::{File, OpenOptions};
use std::io::{Error, IsTerminal as _};
use std::os::unix::{fs::OpenOptionsExt as _, process::CommandExt as _};
use std::process::Command;
use thiserror::Error as ThisError;
use uucore::error::{UError, UResult};
use uucore::translate;

use crate::find_stdout;

#[derive(Debug, ThisError)]
enum PlatformError {
    #[cfg(target_vendor = "apple")]
    #[error("{}", translate!("nohup-error-cannot-detach"))]
    CannotDetach,

    #[error("{}", translate!("nohup-error-cannot-replace", "name" => (*_0), "err" => _1))]
    CannotReplace(&'static str, #[source] Error),
}

impl UError for PlatformError {
    fn code(&self) -> i32 {
        2
    }
}

/// Detach from the controlling terminal: redirect the standard streams and
/// ignore `SIGHUP` so the command survives the shell it was started from.
pub(crate) fn prepare() -> UResult<()> {
    replace_fds()?;

    unsafe { libc::signal(libc::SIGHUP, libc::SIG_IGN) };

    #[cfg(target_vendor = "apple")]
    if unsafe { !_vprocmgr_detach_from_console(0).is_null() } {
        return Err(PlatformError::CannotDetach.into());
    }

    Ok(())
}

/// Replace the current process image with `command`.
///
/// Returns the error that made `execvp` fail; on success it never returns.
#[allow(
    clippy::unnecessary_wraps,
    reason = "signature shared with the Windows implementation, which can fail"
)]
pub(crate) fn run(command: &mut Command) -> UResult<Option<Error>> {
    Ok(Some(command.exec()))
}

/// POSIX nohup creates the output file with mode 0600 so that other users on a
/// shared host can't read whatever the detached job logs.
///
/// This only affects newly-created files; if the file already exists its
/// permissions are left alone.
pub(crate) fn set_output_file_mode(opt: &mut OpenOptions) {
    opt.mode(0o600);
}

fn replace_fds() -> UResult<()> {
    use rustix::stdio::{dup2_stderr, dup2_stdin, dup2_stdout, stdout};
    if std::io::stdin().is_terminal() {
        let new_stdin = File::open(std::path::Path::new("/dev/null"))
            .map_err(|e| PlatformError::CannotReplace("STDIN", e))?;
        dup2_stdin(&new_stdin).map_err(|e| PlatformError::CannotReplace("STDIN", e.into()))?;
    }

    if std::io::stdout().is_terminal() {
        let new_stdout = find_stdout()?;

        dup2_stdout(&new_stdout).map_err(|e| PlatformError::CannotReplace("STDOUT", e.into()))?;
    }

    if std::io::stderr().is_terminal() {
        dup2_stderr(stdout()).map_err(|e| PlatformError::CannotReplace("STDERR", e.into()))?;
    }
    Ok(())
}

#[cfg(target_vendor = "apple")]
unsafe extern "C" {
    fn _vprocmgr_detach_from_console(flags: u32) -> *const core::ffi::c_int;
}
