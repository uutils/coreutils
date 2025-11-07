// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) tstr sigstr cmdname setpgid sigchld getpid
mod status;

use crate::status::ExitStatus;
use clap::{Arg, ArgAction, Command};
use nix::errno::Errno;
use nix::sys::signal::{SigSet, Signal, sigprocmask, SigmaskHow};
use std::io::{self, ErrorKind};
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::process::{self, Child, Stdio};
use std::sync::atomic::{self, AtomicBool};
use std::time::{Duration, Instant};
use uucore::display::Quotable;
use uucore::error::{UResult, USimpleError, UUsageError};
use uucore::parser::parse_time;
use uucore::process::ChildExt;
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

/// We should terminate child process when receiving TERM signal.
/// This is now handled by sigtimedwait() in wait_or_timeout().
static SIGNALED: AtomicBool = AtomicBool::new(false);

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

/// Wait for one of the specified signals to be delivered, with optional timeout.
///
/// This function uses `sigtimedwait()` to efficiently wait for signals without polling.
/// It handles EINTR by retrying the wait.
///
/// # Arguments
/// * `signals` - Signals to wait for (typically SIGCHLD and SIGTERM)
/// * `until` - Optional deadline (absolute time)
///
/// # Returns
/// * `Ok(Some(signal))` - A signal was received
/// * `Ok(None)` - Timeout expired
/// * `Err(e)` - An error occurred
fn wait_for_signal(signals: &[Signal], until: Option<Instant>) -> io::Result<Option<Signal>> {
    // Create signal set from the provided signals
    let mut sigset = SigSet::empty();
    for &sig in signals {
        sigset.add(sig);
    }

    // Retry on EINTR, recalculating timeout each iteration
    loop {
        // Calculate remaining timeout
        let timeout = if let Some(deadline) = until {
            deadline.saturating_duration_since(Instant::now())
        } else {
            Duration::MAX
        };

        // Convert to timespec, handling overflow
        let timeout_spec = if timeout.as_secs() > libc::time_t::MAX as u64 {
            libc::timespec {
                tv_sec: libc::time_t::MAX,
                tv_nsec: 0,
            }
        } else {
            libc::timespec {
                tv_sec: timeout.as_secs() as libc::time_t,
                tv_nsec: timeout.subsec_nanos() as libc::c_long,
            }
        };

        let result = unsafe {
            libc::sigtimedwait(
                sigset.as_ref() as *const libc::sigset_t,
                std::ptr::null_mut(), // We don't need siginfo
                &timeout_spec as *const libc::timespec,
            )
        };

        if result < 0 {
            match Errno::last() {
                // Timeout elapsed with no signal received
                Errno::EAGAIN => return Ok(None),
                // The wait was interrupted by a signal not in our set - retry with recalculated timeout
                Errno::EINTR => continue,
                // Some other error
                err => return Err(io::Error::from(err)),
            }
        } else {
            // Signal received - convert signal number to Signal enum
            return Signal::try_from(result)
                .map(Some)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e));
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
) -> std::io::Result<i32> {
    // ignore `SIGTERM` here
    match process.wait_or_timeout(duration, None) {
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
            send_signal(process, signal, foreground);
            process.wait()?;
            Ok(ExitStatus::SignalSent(signal).into())
        }
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

    // Block signals before spawning child - will be handled by sigtimedwait()
    let mut sigset = SigSet::empty();
    sigset.add(Signal::SIGCHLD);
    sigset.add(Signal::SIGTERM);
    let mut old_sigset = SigSet::empty();
    sigprocmask(SigmaskHow::SIG_BLOCK, Some(&sigset), Some(&mut old_sigset))
        .map_err(|e| USimpleError::new(ExitStatus::TimeoutFailed.into(), e.to_string()))?;

    let process = &mut unsafe {
        process::Command::new(&cmd[0])
            .args(&cmd[1..])
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .pre_exec(|| {
                // Unblock signals that were blocked in parent for sigtimedwait
                // Child needs to receive these signals normally
                let mut unblock_set = SigSet::empty();
                unblock_set.add(Signal::SIGTERM);
                unblock_set.add(Signal::SIGCHLD);
                sigprocmask(SigmaskHow::SIG_UNBLOCK, Some(&unblock_set), None)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
                Ok(())
            })
            .spawn()
    }
        .map_err(|err| {
            // Restore signal mask before returning error
            let _ = sigprocmask(SigmaskHow::SIG_SETMASK, Some(&old_sigset), None);

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

    // Wait for the child process for the specified time period using sigtimedwait.
    // This approach eliminates the 100ms polling delay and provides precise, efficient waiting.
    //
    // The loop combines try_wait() with wait_for_signal() to handle race conditions:
    // - try_wait() checks if the child has already exited
    // - wait_for_signal() suspends until SIGCHLD, SIGTERM, or timeout
    // - On SIGCHLD, we loop back to try_wait() to reap the child
    // - On SIGTERM, we mark SIGNALED and break out (treat as timeout)
    // - On timeout, we break out and send the termination signal

    // .try_wait() doesn't drop stdin, so we do it manually
    drop(process.stdin.take());

    // Handle zero timeout - run command without any timeout
    if duration == Duration::ZERO {
        let exit_status = process.wait().map_err(|e| {
            USimpleError::new(ExitStatus::TimeoutFailed.into(), e.to_string())
        })?;

        // Restore signal mask
        sigprocmask(SigmaskHow::SIG_SETMASK, Some(&old_sigset), None)
            .map_err(|e| USimpleError::new(ExitStatus::TimeoutFailed.into(), e.to_string()))?;

        return match exit_status.code() {
            Some(0) => Ok(()),
            Some(code) => Err(code.into()),
            None => Err(ExitStatus::Terminated.into()),
        };
    }

    let deadline = Instant::now()
        .checked_add(duration)
        .unwrap_or_else(|| Instant::now() + Duration::from_secs(86400 * 365 * 100));
    let wait_result: Option<std::process::ExitStatus> = loop {
        // Wait for signals with timeout
        // If child has already exited, SIGCHLD will be delivered immediately
        let signal_result = wait_for_signal(&[Signal::SIGCHLD, Signal::SIGTERM], Some(deadline));
        match signal_result {
            Ok(Some(Signal::SIGCHLD)) => {
                // Child state changed, reap it
                match process.wait() {
                    Ok(status) => break Some(status),
                    Err(e) => {
                        // Restore mask before returning error
                        let _ = sigprocmask(SigmaskHow::SIG_SETMASK, Some(&old_sigset), None);
                        return Err(e.into());
                    }
                }
            }
            Ok(Some(Signal::SIGTERM)) => {
                // External termination request
                SIGNALED.store(true, atomic::Ordering::Relaxed);
                break None; // Treat as timeout
            }
            Ok(None) => {
                // Timeout expired
                break None;
            }
            Ok(Some(sig)) => {
                // Unexpected signal (shouldn't happen since we only wait for SIGCHLD/SIGTERM)
                let _ = sigprocmask(SigmaskHow::SIG_SETMASK, Some(&old_sigset), None);
                return Err(USimpleError::new(
                    ExitStatus::TimeoutFailed.into(),
                    format!("Unexpected signal received: {:?}", sig),
                ));
            }
            Err(e) => {
                // wait_for_signal failed
                let _ = sigprocmask(SigmaskHow::SIG_SETMASK, Some(&old_sigset), None);
                return Err(e.into());
            }
        }
    };

    let result = match wait_result {
        Some(status) => Err(status
            .code()
            .unwrap_or_else(|| preserve_signal_info(status.signal().unwrap()))
            .into()),
        None => {
            report_if_verbose(signal, &cmd[0], verbose);
            send_signal(process, signal, foreground);
            match kill_after {
                None => {
                    match process.wait() {
                        Ok(status) => {
                            if SIGNALED.load(atomic::Ordering::Relaxed) {
                                Err(ExitStatus::Terminated.into())
                            } else if preserve_status {
                                // When preserve_status is true and timeout occurred:
                                // Special case: SIGCONT doesn't kill, so if it was sent and process
                                // completed successfully, return 0.
                                // All other signals: return 128+signal we sent (not child's status)
                                if signal == libc::SIGCONT.try_into().unwrap() && status.success() {
                                    Ok(())
                                } else {
                                    Err(ExitStatus::SignalSent(signal).into())
                                }
                            } else {
                                Err(ExitStatus::CommandTimedOut.into())
                            }
                        }
                        Err(e) => {
                            let _ = sigprocmask(SigmaskHow::SIG_SETMASK, Some(&old_sigset), None);
                            return Err(e.into());
                        }
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
    };

    // Restore the original signal mask before returning
    // This is CRITICAL - without this, signals stay blocked across invocations!
    let _ = sigprocmask(SigmaskHow::SIG_SETMASK, Some(&old_sigset), None);

    result
}
