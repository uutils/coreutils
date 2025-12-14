// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) SIGHUP cproc vprocmgr homeout

use clap::{Arg, ArgAction, Command};
use libc::{SIG_IGN, SIGHUP, dup2, signal};
use nix::sys::stat::{Mode, umask};
use std::env;
use std::fs::{File, OpenOptions};
use std::io::{Error, ErrorKind, IsTerminal, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::prelude::*;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process;
use thiserror::Error;
use uucore::display::Quotable;
use uucore::error::{ExitCode, UError, UResult, set_exit_code};
use uucore::format_usage;
use uucore::translate;

static NOHUP_OUT: &str = "nohup.out";
// exit codes that match the GNU implementation
static EXIT_CANCELED: i32 = 125;
static EXIT_CANNOT_INVOKE: i32 = 126;
static EXIT_ENOENT: i32 = 127;
static POSIX_NOHUP_FAILURE: i32 = 127;

mod options {
    pub const CMD: &str = "cmd";
}

#[derive(Debug, Error)]
enum NohupError {
    #[error("{}", translate!("nohup-error-cannot-detach"))]
    CannotDetach,

    #[error("{}", translate!("nohup-error-cannot-replace", "name" => (*_0), "err" => _1))]
    CannotReplace(&'static str, #[source] Error),

    #[error("{}", translate!("nohup-error-open-failed", "path" => NOHUP_OUT.quote(), "err" => _1))]
    OpenFailed(i32, #[source] Error),

    #[error("{}", translate!("nohup-error-open-failed-both", "first_path" => NOHUP_OUT.quote(), "first_err" => _1, "second_path" => _2.quote(), "second_err" => _3))]
    OpenFailed2(i32, #[source] Error, String, Error),
}

impl UError for NohupError {
    fn code(&self) -> i32 {
        match self {
            Self::OpenFailed(code, _) | Self::OpenFailed2(code, _, _, _) => *code,
            _ => 2,
        }
    }
}

fn failure_code() -> i32 {
    match env::var("POSIXLY_CORRECT") {
        Ok(_) => POSIX_NOHUP_FAILURE,
        Err(_) => EXIT_CANCELED,
    }
}

/// We are unable to use the regular show_error because we need to detect if stderr
/// is unavailable because GNU nohup exits with 125 if it can't write to stderr.
/// When stderr is unavailable, we use ExitCode to exit silently with the appropriate code.
fn write_stderr(msg: &str) -> UResult<()> {
    let mut stderr = std::io::stderr();
    if writeln!(stderr, "nohup: {msg}").is_err() || stderr.flush().is_err() {
        return Err(ExitCode(failure_code()).into());
    }
    Ok(())
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uucore::clap_localization::handle_clap_result_with_exit_code(
        uu_app(),
        args,
        failure_code(),
    )?;

    replace_fds()?;

    unsafe { signal(SIGHUP, SIG_IGN) };

    if unsafe { !_vprocmgr_detach_from_console(0).is_null() } {
        return Err(NohupError::CannotDetach.into());
    }

    let mut cmd_iter = matches.get_many::<String>(options::CMD).unwrap();
    let cmd = cmd_iter.next().unwrap();
    let args: Vec<&String> = cmd_iter.collect();

    let err = process::Command::new(cmd).args(args).exec();

    match err.kind() {
        ErrorKind::NotFound => set_exit_code(EXIT_ENOENT),
        _ => set_exit_code(EXIT_CANNOT_INVOKE),
    }
    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .about(translate!("nohup-about"))
        .after_help(translate!("nohup-after-help"))
        .override_usage(format_usage(&translate!("nohup-usage")))
        .arg(
            Arg::new(options::CMD)
                .hide(true)
                .required(true)
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::CommandName),
        )
        .trailing_var_arg(true)
        .infer_long_args(true)
}

fn replace_fds() -> UResult<()> {
    let stdin_is_terminal = std::io::stdin().is_terminal();
    let stdout_is_terminal = std::io::stdout().is_terminal();

    if stdin_is_terminal {
        let new_stdin = File::open(Path::new("/dev/null"))
            .map_err(|e| NohupError::CannotReplace("STDIN", e))?;
        if unsafe { dup2(new_stdin.as_raw_fd(), 0) } != 0 {
            return Err(NohupError::CannotReplace("STDIN", Error::last_os_error()).into());
        }
    }

    if stdout_is_terminal {
        let (new_stdout, path) = find_stdout()?;
        let fd = new_stdout.as_raw_fd();

        // Print the appropriate message based on what we're doing
        // Use write_stderr to detect write failures (e.g., /dev/full)
        if stdin_is_terminal {
            write_stderr(&translate!(
                "nohup-ignoring-input-appending-output",
                "path" => path.quote()
            ))?;
        } else {
            write_stderr(&translate!("nohup-appending-output", "path" => path.quote()))?;
        }

        if unsafe { dup2(fd, 1) } != 1 {
            return Err(NohupError::CannotReplace("STDOUT", Error::last_os_error()).into());
        }
    } else if stdin_is_terminal {
        // Only ignoring input, not redirecting stdout
        write_stderr(&translate!("nohup-ignoring-input"))?;
    }

    if std::io::stderr().is_terminal() && unsafe { dup2(1, 2) } != 2 {
        return Err(NohupError::CannotReplace("STDERR", Error::last_os_error()).into());
    }
    Ok(())
}

/// Open nohup.out file with mode 0o600, temporarily clearing umask.
/// The umask is cleared to ensure the file is created with exactly 0o600 permissions.
fn open_nohup_file(path: &Path) -> std::io::Result<File> {
    // Clear umask (set it to 0) and save the old value
    let old_umask = umask(Mode::from_bits_truncate(0));

    let result = OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(path);

    // Restore previous umask
    umask(old_umask);

    result
}

fn find_stdout() -> UResult<(File, String)> {
    let exit_code = failure_code();

    match open_nohup_file(Path::new(NOHUP_OUT)) {
        Ok(t) => Ok((t, NOHUP_OUT.to_string())),
        Err(e1) => {
            let Ok(home) = env::var("HOME") else {
                return Err(NohupError::OpenFailed(exit_code, e1).into());
            };
            let mut homeout = PathBuf::from(home);
            homeout.push(NOHUP_OUT);
            let homeout_str = homeout.to_str().unwrap().to_string();
            match open_nohup_file(&homeout) {
                Ok(t) => Ok((t, homeout_str)),
                Err(e2) => Err(NohupError::OpenFailed2(exit_code, e1, homeout_str, e2).into()),
            }
        }
    }
}

#[cfg(target_vendor = "apple")]
unsafe extern "C" {
    fn _vprocmgr_detach_from_console(flags: u32) -> *const libc::c_int;
}

#[cfg(any(
    target_os = "linux",
    target_os = "android",
    target_os = "freebsd",
    target_os = "openbsd"
))]
/// # Safety
/// This function is unsafe because it dereferences a raw pointer.
unsafe fn _vprocmgr_detach_from_console(_: u32) -> *const libc::c_int {
    std::ptr::null()
}
