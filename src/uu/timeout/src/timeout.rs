//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Alex Lyon <arcterus@mail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) tstr sigstr cmdname setpgid

#[macro_use]
extern crate uucore;

extern crate clap;

use clap::{App, Arg, ArgMatches, AppSettings};
use std::io::ErrorKind;
use std::process::{Command, Stdio};
use std::time::Duration;
use uucore::process::ChildExt;
use uucore::signals::{Signal, signal_by_name_or_value};

 
static NAME: &str = "timeout";
static VERSION: &str = env!("CARGO_PKG_VERSION");

const ERR_EXIT_STATUS: i32 = 125;

pub mod options {
    pub static FOREGROUND: &str = "foreground";
    pub static KILL_AFTER: &str = "kill-after";
    pub static SIGNAL: &str = "signal";
    pub static VERSION: &str = "version";
    pub static PRESERVE_STATUS: &str = "preserve-status";

    // Positional args.
    pub static DURATION: &str = "duration";
    pub static COMMAND: &str = "command";
    pub static ARGS: &str = "args";
}

struct Config {
    foreground: bool,
    kill_after: Option<Duration>,
    signal: Option<Signal>,
    version: bool,
    duration: Duration,
    preserve_status: bool

    command: String,
    command_args: &[String]
}

impl Config {
    fn from(options: Clap::ArgMatches) -> Config {
        let timeout_signal = match options.value_of(options::SIGNAL) {
            Some(signal_) =>
            {
                let signal_result = signal_by_name_or_value(&signal_);
                match signal_result{
                    None => {
                        show_error!("invalid signal '{}'", signal_);
                        return ERR_EXIT_STATUS;
                    },
                    _ => Some(signal_result)
                }
            },
            _ => None
        };

        let kill_after: Option<Duration> =
            match options.value_of(options::KILL_AFTER) {
                Some(time) => Some(uucore::parse_time::from_str(&time)),
                None => None
            };

        let duration: Duration = uucore::parse_time::from_str(
            options.value_of(options::DURATION)
        );

        let preserve_status: bool = options.is_present(options::PRESERVE_STATUS);

        let command: String = options.value_of(options::COMMAND).to_str();
        let command_args: &[String] = options.values_of(options::ARGS)
                                             .map(|x| x.as_str());

        Config {
            foreground: options.is_present(options::FOREGROUND),
            kill_after,
            signal: timeout_signal,
            duration,
            preserve_status,
            command,
            command_args
        }
    }
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let args = args.collect_str();

    let program = args[0].clone();

    let mut opts = getopts::Options::new();

    let mut app = App::new("timeout")
        .version(VERSION)
        .arg(
            Arg::with_name(options::FOREGROUND)
                .long(options::FOREGROUND)
                .help("when not running timeout directly from a shell prompt, allow COMMAND to read from the TTY and get TTY signals; in this mode, children of COMMAND will not be timed out")
        )
        .arg(
            Arg::with_name(options::KILL_AFTER)
                .short("k")
                .takes_value(true))
        .arg(
            Arg::with_name(options::PRESERVE_STATUS)
                .long(options::PRESERVE_STATUS)
                .help("exit with the same status as COMMAND, even when the command times out")
        )
        .arg(
            Arg::with_name(options::SIGNAL)
                .short("s")
                .long(options::SIGNAL)
                .help("specify the signal to be sent on timeout; SIGNAL may be a name like 'HUP' or a number; see 'kill -l' for a list of signals")
                .takes_value(true)
        )
        .arg(
            Arg::with_name(options::DURATION)
                .index(1)
                .required(true)
        )
        .arg(
            Arg::with_name(options::COMMAND)
                .index(2)
                .required(true)
        )
        .arg(
            Arg::with_name(options::ARGS).required(true).multiple(true)
        )
        .setting(AppSettings::TrailingVarArg);

    let matches = app.get_matches_from(args);

    let config = Config::from(matches);
    timeout(config.command,
            config.command_args,
            config.duration,
            config.signal,
            config.kill_after,
            config.foreground,
            config.preserve_status
    )
}

/// TODO: Improve exit codes, and make them consistent with the GNU Coreutil
/// exit codes.

fn timeout(
    cmdname: &str,
    args: &[String],
    duration: Duration,
    signal: usize,
    kill_after: Duration,
    foreground: bool,
    preserve_status: bool,
) -> i32 {
    if !foreground {
        unsafe { libc::setpgid(0, 0) };
    }
    let mut process = match Command::new(cmdname)
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
    {
        Ok(p) => p,
        Err(err) => {
            show_error!("failed to execute process: {}", err);
            if err.kind() == ErrorKind::NotFound {
                // FIXME: not sure which to use
                return 127;
            } else {
                // FIXME: this may not be 100% correct...
                return 126;
            }
        }
    };
    match process.wait_or_timeout(duration) {
        Ok(Some(status)) => status.code().unwrap_or_else(|| status.signal().unwrap()),
        Ok(None) => {
            return_if_err!(ERR_EXIT_STATUS, process.send_signal(signal));
            match process.wait_or_timeout(kill_after) {
                Ok(Some(status)) => {
                    if preserve_status {
                        status.code().unwrap_or_else(|| status.signal().unwrap())
                    } else {
                        124
                    }
                }
                Ok(None) => {
                    if kill_after == Duration::new(0, 0) {
                        // XXX: this may not be right
                        return 124;
                    }
                    return_if_err!(
                        ERR_EXIT_STATUS,
                        process
                            .send_signal(uucore::signals::signal_by_name_or_value("KILL").unwrap())
                    );
                    return_if_err!(ERR_EXIT_STATUS, process.wait());
                    137
                }
                Err(_) => 124,
            }
        }
        Err(_) => {
            return_if_err!(ERR_EXIT_STATUS, process.send_signal(signal));
            ERR_EXIT_STATUS
        }
    }
}
