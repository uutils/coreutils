// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) tstr sigstr cmdname setpgid sigchld getpid TTIN TTOU

mod platform;
mod status;

use crate::status::ExitStatus;
use clap::{Arg, ArgAction, Command};
use std::io::{ErrorKind, Write};
use std::process::{self, Child, Stdio};
use std::sync::atomic::{self, AtomicBool};
use std::time::Duration;
use uucore::display::Quotable;
use uucore::error::{UResult, USimpleError, UUsageError};
use uucore::parser::parse_time;
use uucore::process::ChildExt;
use uucore::translate;

use uucore::{
    format_usage,
    signals::{signal_by_name_or_value, signal_list_name_by_value},
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
        .help_template(uucore::localized_help_template("timeout"))
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

/// We should terminate child process when receiving termination signals.
pub(crate) static SIGNALED: AtomicBool = AtomicBool::new(false);
/// Track which signal was received (0 = none/timeout expired naturally).
#[cfg(unix)]
pub(crate) static RECEIVED_SIGNAL: atomic::AtomicI32 = atomic::AtomicI32::new(0);

/// Report that a signal is being sent if the verbose flag is set.
fn report_if_verbose(signal: usize, cmd: &str, verbose: bool) {
    if verbose {
        let s = if signal == 0 {
            "0".to_string()
        } else {
            signal_list_name_by_value(signal).unwrap()
        };
        let mut stderr = std::io::stderr();
        let _ = writeln!(
            stderr,
            "timeout: {}",
            translate!("timeout-verbose-sending-signal", "signal" => s, "command" => cmd.quote())
        );
        let _ = stderr.flush();
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
    spawn_state: &platform::SpawnState,
) -> std::io::Result<i32> {
    // ignore `SIGTERM` here
    match process.wait_or_timeout(duration, None) {
        Ok(Some(status)) => {
            if preserve_status {
                let exit_code = status.code().unwrap_or_else(|| {
                    platform::status_signal(status).unwrap_or_else(|| {
                        // Extremely rare: process exited but we have neither exit code nor signal.
                        // This can happen on some platforms or in unusual termination scenarios.
                        ExitStatus::TimeoutFailed.into()
                    })
                });
                Ok(exit_code)
            } else {
                Ok(ExitStatus::CommandTimedOut.into())
            }
        }
        Ok(None) => {
            let signal = signal_by_name_or_value("KILL").unwrap();
            report_if_verbose(signal, cmd, verbose);
            platform::send_signal(process, signal, foreground, None, spawn_state);
            process.wait()?;
            Ok(ExitStatus::SignalSent(signal).into())
        }
        Err(_) => Ok(ExitStatus::CommandTimedOut.into()),
    }
}

fn timeout(
    cmd: &[String],
    duration: Duration,
    signal: usize,
    kill_after: Option<Duration>,
    foreground: bool,
    preserve_status: bool,
    verbose: bool,
) -> UResult<()> {
    let mut cmd_builder = process::Command::new(&cmd[0]);
    cmd_builder
        .args(&cmd[1..])
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    platform::prepare(&mut cmd_builder, foreground, signal);

    let process = &mut cmd_builder.spawn().map_err(|err| {
        let status_code = match err.kind() {
            ErrorKind::NotFound => ExitStatus::CommandNotFound.into(),
            ErrorKind::PermissionDenied => ExitStatus::CannotInvoke.into(),
            _ => ExitStatus::CannotInvoke.into(),
        };
        USimpleError::new(
            status_code,
            translate!("timeout-error-failed-to-execute-process", "error" => err),
        )
    })?;

    let spawn_state = platform::post_spawn(process, foreground);

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
    match process.wait_or_timeout(duration, Some(&SIGNALED)) {
        Ok(Some(status)) => {
            let exit_code = status.code().unwrap_or_else(|| {
                platform::status_signal(status).map_or_else(
                    || ExitStatus::TimeoutFailed.into(),
                    platform::preserve_signal_info,
                )
            });
            Err(exit_code.into())
        }
        Ok(None) => {
            // `external_signal()` consumes the latched signal, so read it
            // exactly once: a second read returns `None`, silently flipping
            // the exit code from 128+n to 124.
            let external_signal = platform::external_signal();
            let is_external_signal = external_signal.is_some();
            let signal_to_send = external_signal.unwrap_or(signal);

            report_if_verbose(signal_to_send, &cmd[0], verbose);
            platform::send_signal(
                process,
                signal_to_send,
                foreground,
                external_signal,
                &spawn_state,
            );

            if let Some(kill_after) = kill_after {
                return match wait_or_kill_process(
                    process,
                    &cmd[0],
                    kill_after,
                    preserve_status,
                    foreground,
                    verbose,
                    &spawn_state,
                ) {
                    Ok(status) => Err(status.into()),
                    Err(e) => Err(USimpleError::new(
                        ExitStatus::TimeoutFailed.into(),
                        e.to_string(),
                    )),
                };
            }

            let status = process.wait()?;
            if is_external_signal {
                Err(ExitStatus::SignalSent(signal_to_send).into())
            } else if SIGNALED.load(atomic::Ordering::Relaxed) {
                Err(ExitStatus::CommandTimedOut.into())
            } else if preserve_status {
                Err(status
                    .code()
                    .or_else(|| {
                        platform::status_signal(status)
                            .map(|s| ExitStatus::SignalSent(s as usize).into())
                    })
                    .unwrap_or(ExitStatus::CommandTimedOut.into())
                    .into())
            } else {
                Err(ExitStatus::CommandTimedOut.into())
            }
        }
        Err(_) => {
            // We're going to return ERR_EXIT_STATUS regardless of
            // whether `send_signal()` succeeds or fails
            platform::send_signal(process, signal, foreground, None, &spawn_state);
            Err(ExitStatus::TimeoutFailed.into())
        }
    }
}
