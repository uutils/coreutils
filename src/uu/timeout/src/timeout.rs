//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Alex Lyon <arcterus@mail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) tstr sigstr cmdname setpgid sigchld
mod status;

#[macro_use]
extern crate uucore;

extern crate clap;

use crate::status::ExitStatus;
use clap::{crate_version, Arg, Command};
use std::io::ErrorKind;
use std::process::{self, Child, Stdio};
use std::time::Duration;
use uucore::display::Quotable;
use uucore::error::{UResult, USimpleError};
use uucore::process::ChildExt;
use uucore::signals::{signal_by_name_or_value, signal_name_by_value};
use uucore::{format_usage, InvalidEncodingHandling};

static ABOUT: &str = "Start COMMAND, and kill it if still running after DURATION.";
const USAGE: &str = "{} [OPTION] DURATION COMMAND...";

pub mod options {
    pub static FOREGROUND: &str = "foreground";
    pub static KILL_AFTER: &str = "kill-after";
    pub static SIGNAL: &str = "signal";
    pub static PRESERVE_STATUS: &str = "preserve-status";
    pub static VERBOSE: &str = "verbose";

    // Positional args.
    pub static DURATION: &str = "duration";
    pub static COMMAND: &str = "command";
}

struct Config {
    foreground: bool,
    kill_after: Option<Duration>,
    signal: usize,
    duration: Duration,
    preserve_status: bool,
    verbose: bool,

    command: Vec<String>,
}

impl Config {
    fn from(options: &clap::ArgMatches) -> UResult<Self> {
        let signal = match options.value_of(options::SIGNAL) {
            Some(signal_) => {
                let signal_result = signal_by_name_or_value(signal_);
                match signal_result {
                    None => {
                        unreachable!("invalid signal {}", signal_.quote());
                    }
                    Some(signal_value) => signal_value,
                }
            }
            _ => uucore::signals::signal_by_name_or_value("TERM").unwrap(),
        };

        let kill_after = options
            .value_of(options::KILL_AFTER)
            .map(|time| uucore::parse_time::from_str(time).unwrap());

        let duration =
            match uucore::parse_time::from_str(options.value_of(options::DURATION).unwrap()) {
                Ok(duration) => duration,
                Err(err) => return Err(USimpleError::new(1, err)),
            };

        let preserve_status: bool = options.is_present(options::PRESERVE_STATUS);
        let foreground = options.is_present(options::FOREGROUND);
        let verbose = options.is_present(options::VERBOSE);

        let command = options
            .values_of(options::COMMAND)
            .unwrap()
            .map(String::from)
            .collect::<Vec<_>>();

        Ok(Self {
            foreground,
            kill_after,
            signal,
            duration,
            preserve_status,
            verbose,
            command,
        })
    }
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args = args
        .collect_str(InvalidEncodingHandling::ConvertLossy)
        .accept_any();

    let command = uu_app();

    let matches = command.get_matches_from(args);

    let config = Config::from(&matches)?;
    timeout(
        &config.command,
        config.duration,
        config.signal,
        config.kill_after,
        config.foreground,
        config.preserve_status,
        config.verbose,
    )
}

pub fn uu_app<'a>() -> Command<'a> {
    Command::new("timeout")
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .arg(
            Arg::new(options::FOREGROUND)
                .long(options::FOREGROUND)
                .help("when not running timeout directly from a shell prompt, allow COMMAND to read from the TTY and get TTY signals; in this mode, children of COMMAND will not be timed out")
        )
        .arg(
            Arg::new(options::KILL_AFTER)
                .short('k')
                .takes_value(true))
        .arg(
            Arg::new(options::PRESERVE_STATUS)
                .long(options::PRESERVE_STATUS)
                .help("exit with the same status as COMMAND, even when the command times out")
        )
        .arg(
            Arg::new(options::SIGNAL)
                .short('s')
                .long(options::SIGNAL)
                .help("specify the signal to be sent on timeout; SIGNAL may be a name like 'HUP' or a number; see 'kill -l' for a list of signals")
                .takes_value(true)
        )
        .arg(
            Arg::new(options::VERBOSE)
              .short('v')
              .long(options::VERBOSE)
              .help("diagnose to stderr any signal sent upon timeout")
        )
        .arg(
            Arg::new(options::DURATION)
                .index(1)
                .required(true)
        )
        .arg(
            Arg::new(options::COMMAND)
                .index(2)
                .required(true)
                .multiple_occurrences(true)
        )
        .trailing_var_arg(true)
        .infer_long_args(true)
}

/// Remove pre-existing SIGCHLD handlers that would make waiting for the child's exit code fail.
fn unblock_sigchld() {
    unsafe {
        nix::sys::signal::signal(
            nix::sys::signal::Signal::SIGCHLD,
            nix::sys::signal::SigHandler::SigDfl,
        )
        .unwrap();
    }
}

/// Report that a signal is being sent if the verbose flag is set.
fn report_if_verbose(signal: usize, cmd: &str, verbose: bool) {
    if verbose {
        let s = signal_name_by_value(signal).unwrap();
        show_error!("sending signal {} to command {}", s, cmd.quote());
    }
}

/// Wait for a child process and send a kill signal if it does not terminate.
///
/// This function waits for the child `process` for the time period
/// given by `duration`. If the child process does not terminate
/// within that time, we send the `SIGKILL` signal to it. If `verbose`
/// is `true`, then a message is printed to `stderr` when that
/// happens.
///
/// If the child process terminates within the given time period and
/// `preserve_status` is `true`, then the status code of the child
/// process is returned. If the child process terminates within the
/// given time period and `preserve_status` is `false`, then 124 is
/// returned. If the child does not terminate within the time period,
/// then 137 is returned. Finally, if there is an error while waiting
/// for the child process to terminate, then 124 is returned.
///
/// # Errors
///
/// If there is a problem sending the `SIGKILL` signal or waiting for
/// the process after that signal is sent.
fn wait_or_kill_process(
    mut process: Child,
    cmd: &str,
    duration: Duration,
    preserve_status: bool,
    verbose: bool,
) -> std::io::Result<i32> {
    match process.wait_or_timeout(duration) {
        Ok(Some(status)) => {
            if preserve_status {
                Ok(status.code().unwrap_or_else(|| status.signal().unwrap()))
            } else {
                Ok(ExitStatus::TimeoutFailed.into())
            }
        }
        Ok(None) => {
            let signal = signal_by_name_or_value("KILL").unwrap();
            report_if_verbose(signal, cmd, verbose);
            process.send_signal(signal)?;
            process.wait()?;
            Ok(ExitStatus::SignalSent(signal).into())
        }
        Err(_) => Ok(ExitStatus::WaitingFailed.into()),
    }
}

/// TODO: Improve exit codes, and make them consistent with the GNU Coreutils exit codes.

fn timeout(
    cmd: &[String],
    duration: Duration,
    signal: usize,
    kill_after: Option<Duration>,
    foreground: bool,
    preserve_status: bool,
    verbose: bool,
) -> UResult<()> {
    if !foreground {
        unsafe { libc::setpgid(0, 0) };
    }
    let mut process = process::Command::new(&cmd[0])
        .args(&cmd[1..])
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|err| {
            let status_code = if err.kind() == ErrorKind::NotFound {
                // FIXME: not sure which to use
                127
            } else {
                // FIXME: this may not be 100% correct...
                126
            };
            USimpleError::new(status_code, format!("failed to execute process: {}", err))
        })?;
    unblock_sigchld();
    // Wait for the child process for the specified time period.
    //
    // If the process exits within the specified time period (the
    // `Ok(Some(_))` arm), then return the appropriate status code.
    //
    // If the process does not exit within that time (the `Ok(None)`
    // arm) and `kill_after` is specified, then try sending `SIGKILL`.
    //
    // TODO The structure of this block is extremely similar to the
    // structure of `wait_or_kill_process()`. They can probably be
    // refactored into some common function.
    match process.wait_or_timeout(duration) {
        Ok(Some(status)) => Err(status
            .code()
            .unwrap_or_else(|| status.signal().unwrap())
            .into()),
        Ok(None) => {
            report_if_verbose(signal, &cmd[0], verbose);
            process.send_signal(signal)?;
            match kill_after {
                None => {
                    if preserve_status {
                        Err(ExitStatus::SignalSent(signal).into())
                    } else {
                        Err(ExitStatus::CommandTimedOut.into())
                    }
                }
                Some(kill_after) => {
                    match wait_or_kill_process(
                        process,
                        &cmd[0],
                        kill_after,
                        preserve_status,
                        verbose,
                    ) {
                        Ok(status) => Err(status.into()),
                        Err(e) => Err(USimpleError::new(
                            ExitStatus::TimeoutFailed.into(),
                            format!("{}", e),
                        )),
                    }
                }
            }
        }
        Err(_) => {
            // We're going to return ERR_EXIT_STATUS regardless of
            // whether `send_signal()` succeeds or fails, so just
            // ignore the return value.
            process.send_signal(signal).map_err(|e| {
                USimpleError::new(ExitStatus::TimeoutFailed.into(), format!("{}", e))
            })?;
            Err(ExitStatus::TimeoutFailed.into())
        }
    }
}
