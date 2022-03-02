//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Alex Lyon <arcterus@mail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) tstr sigstr cmdname setpgid sigchld

#[macro_use]
extern crate uucore;

extern crate clap;

use clap::{crate_version, App, AppSettings, Arg};
use std::io::ErrorKind;
use std::process::{Command, Stdio};
use std::time::Duration;
use uucore::display::Quotable;
use uucore::error::{UResult, USimpleError};
use uucore::process::ChildExt;
use uucore::signals::{signal_by_name_or_value, signal_name_by_value};
use uucore::{format_usage, InvalidEncodingHandling};

static ABOUT: &str = "Start COMMAND, and kill it if still running after DURATION.";
const USAGE: &str = "{} [OPTION] DURATION COMMAND...";

const ERR_EXIT_STATUS: i32 = 125;

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

    let app = uu_app();

    let matches = app.get_matches_from(args);

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

pub fn uu_app<'a>() -> App<'a> {
    App::new("timeout")
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
        .setting(AppSettings::TrailingVarArg)
        .setting(AppSettings::InferLongArgs)
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
    let mut process = Command::new(&cmd[0])
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
    match process.wait_or_timeout(duration) {
        Ok(Some(status)) => {
            let status_code = status.code().unwrap_or_else(|| status.signal().unwrap());
            if status_code == 0 {
                Ok(())
            } else {
                Err(status_code.into())
            }
        }
        Ok(None) => {
            if verbose {
                show_error!(
                    "sending signal {} to command {}",
                    signal_name_by_value(signal).unwrap(),
                    cmd[0].quote()
                );
            }
            process
                .send_signal(signal)
                .map_err(|e| USimpleError::new(ERR_EXIT_STATUS, format!("{}", e)))?;
            if let Some(kill_after) = kill_after {
                match process.wait_or_timeout(kill_after) {
                    Ok(Some(status)) => {
                        if preserve_status {
                            let status_code =
                                status.code().unwrap_or_else(|| status.signal().unwrap());
                            if status_code == 0 {
                                Ok(())
                            } else {
                                Err(status_code.into())
                            }
                        } else {
                            Err(124.into())
                        }
                    }
                    Ok(None) => {
                        if verbose {
                            show_error!("sending signal KILL to command {}", cmd[0].quote());
                        }
                        process
                            .send_signal(uucore::signals::signal_by_name_or_value("KILL").unwrap())
                            .map_err(|e| USimpleError::new(ERR_EXIT_STATUS, format!("{}", e)))?;
                        process
                            .wait()
                            .map_err(|e| USimpleError::new(ERR_EXIT_STATUS, format!("{}", e)))?;
                        Err(137.into())
                    }
                    Err(_) => Err(124.into()),
                }
            } else {
                Err(124.into())
            }
        }
        Err(_) => {
            // We're going to return ERR_EXIT_STATUS regardless of
            // whether `send_signal()` succeeds or fails, so just
            // ignore the return value.
            process
                .send_signal(signal)
                .map_err(|e| USimpleError::new(ERR_EXIT_STATUS, format!("{}", e)))?;
            Err(ERR_EXIT_STATUS.into())
        }
    }
}
