// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) tstr sigstr cmdname setpgid sigchld getpid
mod status;

use crate::status::{TimeoutError, TimeoutResult};
use clap::{Arg, ArgAction, Command};
use nix::errno::Errno;
use nix::sys::signal::{SigHandler, SigSet, SigmaskHow, Signal, kill, sigprocmask};
use nix::sys::time::{TimeSpec, time_t};
use nix::unistd::Pid;
use std::io::{self, ErrorKind};
use std::os::unix::process::ExitStatusExt;
use std::process::{self, Child, ExitStatus, Stdio};
use std::time::{Duration, Instant};
use uucore::display::Quotable;
use uucore::error::{UResult, UUsageError};
use uucore::parser::parse_time;
use uucore::process::ChildExt;
use uucore::signals::enable_pipe_errors;
use uucore::{
    format_usage, show_error,
    signals::{signal_by_name_or_value, signal_name_by_value},
    translate,
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
            Some(signal) => signal_by_name_or_value(signal).ok_or_else(|| {
                UUsageError::new(
                    1,
                    translate!("timeout-error-invalid-signal", "signal" => signal.quote()),
                )
            })?,
            None => Signal::SIGTERM as usize,
        };

        let kill_after = match options.get_one::<String>(options::KILL_AFTER) {
            None => None,
            Some(kill_after) => match parse_time::from_str(kill_after, true) {
                Ok(k) => Some(k),
                Err(err) => return Err(UUsageError::new(1, err)),
            },
        };

        let duration =
            parse_time::from_str(options.get_one::<String>(options::DURATION).unwrap(), true)
                .map_err(|err| UUsageError::new(1, err))?;

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
    let matches = uucore::clap_localization::handle_clap_result_with_exit_code(uu_app(), args, 125)
        .map_err(TimeoutError::from)?;
    let config = Config::from(&matches).map_err(TimeoutError::from)?;

    let status = timeout(&config)?;

    exit(status.to_exit_status(config.preserve_status)).map_err(TimeoutError::from)?;
    Ok(())
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
fn report_if_verbose(signal: usize, cmd: &str, config: &Config) {
    if config.verbose {
        let signal = signal_name_by_value(signal).expect("unsupported signal");
        show_error!(
            "{}",
            translate!("timeout-verbose-sending-signal", "signal" => signal, "command" => cmd.quote())
        );
    }
}

fn block_signals(signals: impl IntoIterator<Item = Signal>) -> io::Result<()> {
    let set = SigSet::from_iter(signals);
    sigprocmask(SigmaskHow::SIG_BLOCK, Some(&set), None)?;
    Ok(())
}

fn unblock_signals(signals: impl IntoIterator<Item = Signal>) -> io::Result<()> {
    let set = SigSet::from_iter(signals);
    sigprocmask(SigmaskHow::SIG_UNBLOCK, Some(&set), None)?;
    Ok(())
}

fn send_signal(process: &mut Child, signal: usize, config: &Config) {
    // NOTE: GNU timeout doesn't check for errors of signal.
    // The subprocess might have exited just after the timeout.
    // Sending a signal now would return "No such process", but we should still try to kill the children.
    if config.foreground {
        let _ = process.send_signal(signal);
    } else {
        if let Ok(signal) = Signal::try_from(signal as i32) {
            let _ = unblock_signals([signal]);
        }
        let _ = process.send_signal_group(signal);
        if signal != Signal::SIGKILL as usize && signal != Signal::SIGCONT as usize {
            let _ = process.send_signal_group(Signal::SIGCONT as usize);
        }
    }
}

fn wait_for_signal(
    signals: impl IntoIterator<Item = Signal>,
    until: Option<Instant>,
) -> io::Result<Option<Signal>> {
    let set = SigSet::from_iter(signals);

    let timeout = until
        .map(|until| until.saturating_duration_since(Instant::now()))
        .unwrap_or(Duration::MAX);
    let mut timeout = TimeSpec::from_duration(timeout);

    // Note that `TimeSpec::from_duration` can silently overflow when passing a duration that is
    // too long. If that happens, `tv_sec` (which is signed) will be < 0
    if timeout.tv_sec() < 0 {
        timeout = TimeSpec::new(time_t::MAX, 0);
    }

    loop {
        let result = unsafe {
            libc::sigtimedwait(
                &raw const *set.as_ref(),
                std::ptr::null_mut(),
                &raw const *timeout.as_ref(),
            )
        };

        if result < 0 {
            match Errno::last() {
                // Timeout elapsed with no signal received
                Errno::EAGAIN => break Ok(None),
                // The wait was interrupted by a signal sent to this process and needs to be retried
                Errno::EINTR => {}
                // Some error occurred
                err => break Err(err.into()),
            }
        } else {
            break Signal::try_from(result).map(Some).map_err(|err| err.into());
        }
    }
}

fn wait_for_exit(
    child: &mut Child,
    mut timeout: Duration,
    config: &Config,
) -> io::Result<Option<process::ExitStatus>> {
    if timeout.is_zero() {
        timeout = Duration::MAX;
    }

    let until = Instant::now().checked_add(timeout);

    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(Some(status));
        }

        match wait_for_signal([Signal::SIGCHLD, Signal::SIGTERM], until)? {
            // The child process has terminated, was stopped, or was resumed. Continue the loop to
            // see if it has stopped, otherwise wait again.
            Some(Signal::SIGCHLD) => continue,
            // This process has received a signal which needs to be forwarded to the child process.
            Some(Signal::SIGTERM) => send_signal(child, Signal::SIGTERM as usize, config),
            // The wait timed out.
            None => return Ok(None),
            // This should not happen.
            Some(signal) => unreachable!(
                "wait_for_signal() returned a signal that was not requested: {signal:?}"
            ),
        }
    }
}

fn exit(status: ExitStatus) -> io::Result<()> {
    if let Some(code) = status.code() {
        process::exit(code);
    } else if let Some(signal) = status.signal() {
        let signal = Signal::try_from(signal)?;
        if signal != Signal::SIGKILL {
            unblock_signals([signal])?;
            unsafe {
                nix::sys::signal::signal(signal, SigHandler::SigDfl)?;
            }
        }
        kill(Pid::this(), Some(signal))?;
        unreachable!("kill() should have terminated this process");
    } else {
        unreachable!("exit status was neither a code nor a signal");
    }
}

fn timeout(config: &Config) -> Result<TimeoutResult, TimeoutError> {
    if !config.foreground {
        unsafe { libc::setpgid(0, 0) };
    }

    enable_pipe_errors()?;

    let mut child = match process::Command::new(&config.command[0])
        .args(&config.command[1..])
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
    {
        Ok(child) => child,
        Err(err) if err.kind() == ErrorKind::NotFound => {
            return Err(TimeoutError::CommandNotFound(err).into());
        }
        Err(err) => return Err(TimeoutError::CommandFailedInvocation(err).into()),
    };

    block_signals([Signal::SIGCHLD, Signal::SIGTERM])?;

    if let Some(status) = wait_for_exit(&mut child, config.duration, config)? {
        return Ok(TimeoutResult::Exited(status));
    }

    // Timeout exceeded; notify the child process
    report_if_verbose(config.signal, &config.command[0], config);
    send_signal(&mut child, config.signal, config);

    if let Some(kill_after) = config.kill_after {
        if let Some(status) = wait_for_exit(&mut child, kill_after, config)? {
            return Ok(TimeoutResult::TimedOut(status));
        }

        // `kill_after` timeout exceeded; kill the child process
        report_if_verbose(Signal::SIGKILL as usize, &config.command[0], config);
        send_signal(&mut child, Signal::SIGKILL as usize, config);
    }

    let status = child.wait()?;
    Ok(TimeoutResult::TimedOut(status))
}
