// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) tstr sigstr cmdname setpgid sigchld getpid
mod status;

use crate::status::ExitStatus;
use clap::{Arg, ArgAction, Command};
use nix::sys::signal::Signal;
use std::io::ErrorKind;
use std::os::unix::process::ExitStatusExt;
use std::process::{self, Child, Stdio};
use std::time::Duration;
use uucore::display::Quotable;
use uucore::error::{UResult, USimpleError, UUsageError};
use uucore::parser::parse_time;
use uucore::process::{ChildExt, CommandExt, SelfPipe, WaitOrTimeoutRet};
use uucore::translate;

#[cfg(unix)]
use uucore::signals::enable_pipe_errors;

use uucore::{
    format_usage, show_error,
    signals::{signal_by_name_or_value, signal_name_by_value},
};

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
        let signal = match options.get_one::<String>(options::SIGNAL) {
            Some(signal_) => {
                let signal_result = signal_by_name_or_value(signal_);
                match signal_result {
                    None => {
                        return Err(UUsageError::new(
                            ExitStatus::TimeoutFailed.into(),
                            translate!("timeout-error-invalid-signal", "signal" => signal_.quote()),
                        ));
                    }
                    Some(signal_value) => signal_value,
                }
            }
            _ => signal_by_name_or_value("TERM").unwrap(),
        };

        let kill_after = match options.get_one::<String>(options::KILL_AFTER) {
            None => None,
            Some(kill_after) => match parse_time::from_str(kill_after, true) {
                Ok(k) => Some(k),
                Err(err) => return Err(UUsageError::new(ExitStatus::TimeoutFailed.into(), err)),
            },
        };

        let duration =
            parse_time::from_str(options.get_one::<String>(options::DURATION).unwrap(), true)
                .map_err(|err| UUsageError::new(ExitStatus::TimeoutFailed.into(), err))?;

        let preserve_status: bool = options.get_flag(options::PRESERVE_STATUS);
        let foreground = options.get_flag(options::FOREGROUND);
        let verbose = options.get_flag(options::VERBOSE);

        let command = options
            .get_many::<String>(options::COMMAND)
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
    let matches =
        uucore::clap_localization::handle_clap_result_with_exit_code(uu_app(), args, 125)?;

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

pub fn uu_app() -> Command {
    Command::new("timeout")
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .about(translate!("timeout-about"))
        .override_usage(format_usage(&translate!("timeout-usage")))
        .arg(
            Arg::new(options::FOREGROUND)
                .long(options::FOREGROUND)
                .short('f')
                .help(translate!("timeout-help-foreground"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::KILL_AFTER)
                .long(options::KILL_AFTER)
                .short('k')
                .help(translate!("timeout-help-kill-after")),
        )
        .arg(
            Arg::new(options::PRESERVE_STATUS)
                .long(options::PRESERVE_STATUS)
                .short('p')
                .help(translate!("timeout-help-preserve-status"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SIGNAL)
                .short('s')
                .long(options::SIGNAL)
                .help(translate!("timeout-help-signal"))
                .value_name("SIGNAL"),
        )
        .arg(
            Arg::new(options::VERBOSE)
                .short('v')
                .long(options::VERBOSE)
                .help(translate!("timeout-help-verbose"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::DURATION)
                .required(true)
                .help(translate!("timeout-help-duration")),
        )
        .arg(
            Arg::new(options::COMMAND)
                .required(true)
                .action(ArgAction::Append)
                .help(translate!("timeout-help-command"))
                .value_hint(clap::ValueHint::CommandName),
        )
        .trailing_var_arg(true)
        .infer_long_args(true)
        .after_help(translate!("timeout-after-help"))
}

/// Report that a signal is being sent if the verbose flag is set.
fn report_if_verbose(signal: usize, cmd: &str, verbose: bool) {
    if verbose {
        let s = signal_name_by_value(signal).unwrap();
        show_error!(
            "{}",
            translate!("timeout-verbose-sending-signal", "signal" => s, "command" => cmd.quote())
        );
    }
}

fn send_signal(process: &mut Child, signal: usize, foreground: bool) {
    // NOTE: GNU timeout doesn't check for errors of signal.
    // The subprocess might have exited just after the timeout.
    // Sending a signal now would return "No such process", but we should still try to kill the children.
    if foreground {
        let _ = process.send_signal(signal);
    } else {
        let _ = process.send_signal_group(signal);
        let kill_signal = signal_by_name_or_value("KILL").unwrap();
        let continued_signal = signal_by_name_or_value("CONT").unwrap();
        if signal != kill_signal && signal != continued_signal {
            _ = process.send_signal_group(continued_signal);
        }
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
    process: &mut Child,
    cmd: &str,
    duration: Duration,
    preserve_status: bool,
    foreground: bool,
    verbose: bool,
    self_pipe: &mut SelfPipe,
) -> std::io::Result<i32> {
    // ignore `SIGTERM` here
    self_pipe.unset_other()?;

    match process.wait_or_timeout(duration, self_pipe) {
        Ok(WaitOrTimeoutRet::InTime(status)) => {
            if preserve_status {
                Ok(status.code().unwrap_or_else(|| status.signal().unwrap()))
            } else {
                Ok(ExitStatus::TimeoutFailed.into())
            }
        }
        Ok(WaitOrTimeoutRet::TimedOut) => {
            let signal = signal_by_name_or_value("KILL").unwrap();
            report_if_verbose(signal, cmd, verbose);
            send_signal(process, signal, foreground);
            process.wait()?;
            Ok(ExitStatus::SignalSent(signal).into())
        }
        Ok(WaitOrTimeoutRet::CustomSignaled) => unreachable!(), // We did not set it up.
        Err(_) => Ok(ExitStatus::WaitingFailed.into()),
    }
}

#[cfg(unix)]
fn preserve_signal_info(signal: libc::c_int) -> libc::c_int {
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
    unsafe {
        libc::kill(libc::getpid(), signal);
    }
    signal
}

#[cfg(not(unix))]
fn preserve_signal_info(signal: libc::c_int) -> libc::c_int {
    // Do nothing
    signal
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
    #[cfg(unix)]
    enable_pipe_errors()?;

    let mut command = process::Command::new(&cmd[0]);
    command
        .args(&cmd[1..])
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    let mut self_pipe = command.set_up_timeout(Some(Signal::SIGTERM))?;
    let process = &mut command.spawn().map_err(|err| {
        let status_code = if err.kind() == ErrorKind::NotFound {
            // FIXME: not sure which to use
            127
        } else {
            // FIXME: this may not be 100% correct...
            126
        };
        USimpleError::new(
            status_code,
            translate!("timeout-error-failed-to-execute-process", "error" => err),
        )
    })?;
    // Wait for the child process for the specified time period.
    //
    // TODO The structure of this block is extremely similar to the
    // structure of `wait_or_kill_process()`. They can probably be
    // refactored into some common function.
    match process.wait_or_timeout(duration, &mut self_pipe) {
        Ok(WaitOrTimeoutRet::InTime(status)) => Err(status
            .code()
            .unwrap_or_else(|| preserve_signal_info(status.signal().unwrap()))
            .into()),
        Ok(WaitOrTimeoutRet::CustomSignaled) => {
            report_if_verbose(signal, &cmd[0], verbose);
            send_signal(process, signal, foreground);
            process.wait()?;
            Err(ExitStatus::Terminated.into())
        }
        Ok(WaitOrTimeoutRet::TimedOut) => {
            report_if_verbose(signal, &cmd[0], verbose);
            send_signal(process, signal, foreground);
            match kill_after {
                None => {
                    let status = process.wait()?;
                    if preserve_status {
                        if let Some(ec) = status.code() {
                            Err(ec.into())
                        } else if let Some(sc) = status.signal() {
                            Err(ExitStatus::SignalSent(sc.try_into().unwrap()).into())
                        } else {
                            Err(ExitStatus::CommandTimedOut.into())
                        }
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
                        foreground,
                        verbose,
                        &mut self_pipe,
                    ) {
                        Ok(status) => Err(status.into()),
                        Err(e) => Err(USimpleError::new(
                            ExitStatus::TimeoutFailed.into(),
                            e.to_string(),
                        )),
                    }
                }
            }
        }
        Err(_) => {
            // We're going to return ERR_EXIT_STATUS regardless of
            // whether `send_signal()` succeeds or fails
            send_signal(process, signal, foreground);
            Err(ExitStatus::TimeoutFailed.into())
        }
    }
}
