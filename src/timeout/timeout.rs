#![crate_name = "uu_timeout"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Arcterus <arcterus@mail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate getopts;
extern crate libc;
extern crate time;

#[macro_use]
extern crate uucore;

use std::io::{ErrorKind, Write};
use std::process::{Command, Stdio};
use std::time::Duration;
use uucore::process::ChildExt;

static NAME: &'static str = "timeout";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

static ERR_EXIT_STATUS: i32 = 125;

pub fn uumain(args: Vec<String>) -> i32 {
    let program = args[0].clone();

    let mut opts = getopts::Options::new();
    opts.optflag("", "preserve-status", "exit with the same status as COMMAND, even when the command times out");
    opts.optflag("", "foreground", "when not running timeout directly from a shell prompt, allow COMMAND to read from the TTY and get TTY signals; in this mode, children of COMMAND will not be timed out");
    opts.optopt("k", "kill-after", "also send a KILL signal if COMMAND is still running this long after the initial signal was sent", "DURATION");
    opts.optflag("s", "signal", "specify the signal to be sent on timeout; SIGNAL may be a name like 'HUP' or a number; see 'kill -l' for a list of signals");
    opts.optflag("h", "help", "display this help and exit");
    opts.optflag("V", "version", "output version information and exit");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            crash!(ERR_EXIT_STATUS, "{}", f)
        }
    };
    if matches.opt_present("help") {
        print!("{} {}

Usage:
  {} [OPTION] DURATION COMMAND [ARG]...

{}", NAME, VERSION, program, &opts.usage("Start COMMAND, and kill it if still running after DURATION."));
    } else if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
    } else if matches.free.len() < 2 {
        show_error!("missing an argument");
        show_error!("for help, try '{0} --help'", program);
        return ERR_EXIT_STATUS;
    } else {
        let status = matches.opt_present("preserve-status");
        let foreground = matches.opt_present("foreground");
        let kill_after = match matches.opt_str("kill-after") {
            Some(tstr) => match uucore::parse_time::from_str(&tstr) {
                Ok(time) => time,
                Err(f) => {
                    show_error!("{}", f);
                    return ERR_EXIT_STATUS;
                }
            },
            None => Duration::new(0, 0),
        };
        let signal = match matches.opt_str("signal") {
            Some(sigstr) => match uucore::signals::signal_by_name_or_value(&sigstr) {
                Some(sig) => sig,
                None => {
                    show_error!("invalid signal '{}'", sigstr);
                    return ERR_EXIT_STATUS;
                }
            },
            None => uucore::signals::signal_by_name_or_value("TERM").unwrap()
        };
        let duration = match uucore::parse_time::from_str(&matches.free[0]) {
            Ok(time) => time,
            Err(f) => {
                show_error!("{}", f);
                return ERR_EXIT_STATUS;
            }
        };
        return timeout(&matches.free[1], &matches.free[2..], duration, signal, kill_after, foreground, status);
    }

    0
}

fn timeout(cmdname: &str, args: &[String], duration: Duration, signal: usize, kill_after: Duration, foreground: bool, preserve_status: bool) -> i32 {
    if !foreground {
        unsafe { libc::setpgid(0, 0) };
    }
    let mut process = match Command::new(cmdname).args(args)
                                                 .stdin(Stdio::inherit())
                                                 .stdout(Stdio::inherit())
                                                 .stderr(Stdio::inherit())
                                                 .spawn() {
        Ok(p) => p,
        Err(err) => {
            show_error!("failed to execute process: {}", err);
            if err.kind() == ErrorKind::NotFound {
                // XXX: not sure which to use
                return 127;
            } else {
                // XXX: this may not be 100% correct...
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
                },
                Ok(None) => {
                    if kill_after == Duration::new(0, 0) {
                        // XXX: this may not be right
                        return 124;
                    }
                    return_if_err!(ERR_EXIT_STATUS, process.send_signal(uucore::signals::signal_by_name_or_value("KILL").unwrap()));
                    return_if_err!(ERR_EXIT_STATUS, process.wait());
                    137
                },
                Err(_) => 124,
            }
        },
        Err(_) => {
            return_if_err!(ERR_EXIT_STATUS, process.send_signal(signal));
            ERR_EXIT_STATUS
        },
    }
}
