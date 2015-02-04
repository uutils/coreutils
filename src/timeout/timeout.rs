#![crate_name = "timeout"]
#![feature(collections, core, io, libc, rustc_private)]

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

use std::old_io::{PathDoesntExist, FileNotFound};
use std::old_io::process::{Command, ExitStatus, ExitSignal, InheritFd};

#[path = "../common/util.rs"]
#[macro_use]
mod util;

#[path = "../common/time.rs"]
mod time;

#[path = "../common/signals.rs"]
mod signals;

extern {
    pub fn setpgid(_: libc::pid_t, _: libc::pid_t) -> libc::c_int;
}

static NAME: &'static str = "timeout";
static VERSION: &'static str = "1.0.0";

static ERR_EXIT_STATUS: isize = 125;

pub fn uumain(args: Vec<String>) -> isize {
    let program = args[0].clone();

    let opts = [
        getopts::optflag("", "preserve-status", "exit with the same status as COMMAND, even when the command times out"),
        getopts::optflag("", "foreground", "when not running timeout directly from a shell prompt, allow COMMAND to read from the TTY and get TTY signals; in this mode, children of COMMAND will not be timed out"),
        getopts::optopt("k", "kill-after", "also send a KILL signal if COMMAND is still running this long after the initial signal was sent", "DURATION"),
        getopts::optflag("s", "signal", "specify the signal to be sent on timeout; SIGNAL may be a name like 'HUP' or a number; see 'kill -l' for a list of signals"),
        getopts::optflag("h", "help", "display this help and exit"),
        getopts::optflag("V", "version", "output version information and exit")
    ];
    let matches = match getopts::getopts(args.tail(), &opts) {
        Ok(m) => m,
        Err(f) => {
            crash!(ERR_EXIT_STATUS, "{}", f)
        }
    };
    if matches.opt_present("help") {
        print!("{} v{}

Usage:
  {} [OPTION] DURATION COMMAND [ARG]...

{}", NAME, VERSION, program, getopts::usage("Start COMMAND, and kill it if still running after DURATION.", &opts));
    } else if matches.opt_present("version") {
        println!("{} v{}", NAME, VERSION);
    } else if matches.free.len() < 2 {
        show_error!("missing an argument");
        show_error!("for help, try '{0} --help'", program);
        return ERR_EXIT_STATUS;
    } else {
        let status = matches.opt_present("preserve-status");
        let foreground = matches.opt_present("foreground");
        let kill_after = match matches.opt_str("kill-after") {
            Some(tstr) => match time::from_str(tstr.as_slice()) {
                Ok(time) => time,
                Err(f) => {
                    show_error!("{}", f);
                    return ERR_EXIT_STATUS;
                }
            },
            None => 0f64
        };
        let signal = match matches.opt_str("signal") {
            Some(sigstr) => match signals::signal_by_name_or_value(sigstr.as_slice()) {
                Some(sig) => sig,
                None => {
                    show_error!("invalid signal '{}'", sigstr);
                    return ERR_EXIT_STATUS;
                }
            },
            None => signals::signal_by_name_or_value("TERM").unwrap()
        };
        let duration = match time::from_str(matches.free[0].as_slice()) {
            Ok(time) => time,
            Err(f) => {
                show_error!("{}", f);
                return ERR_EXIT_STATUS;
            }
        };
        return timeout(matches.free[1].as_slice(), &matches.free[2..], duration, signal, kill_after, foreground, status);
    }

    0
}

fn timeout(cmdname: &str, args: &[String], duration: f64, signal: usize, kill_after: f64, foreground: bool, preserve_status: bool) -> isize {
    if !foreground {
        unsafe { setpgid(0, 0) };
    }
    let mut process = match Command::new(cmdname).args(args)
                                                 .stdin(InheritFd(0))
                                                 .stdout(InheritFd(1))
                                                 .stderr(InheritFd(2))
                                                 .spawn() {
        Ok(p) => p,
        Err(err) => {
            show_error!("failed to execute process: {}", err);
            if err.kind == FileNotFound || err.kind == PathDoesntExist {
                // XXX: not sure which to use
                return 127;
            } else {
                // XXX: this may not be 100% correct...
                return 126;
            }
        }
    };
    process.set_timeout(Some((duration * 1000f64) as u64));  // FIXME: this ignores the f64...
    match process.wait() {
        Ok(status) => match status {
            ExitStatus(stat) => stat,
            ExitSignal(stat) => stat
        },
        Err(_) => {
            return_if_err!(ERR_EXIT_STATUS, process.signal(signal as isize));
            process.set_timeout(Some((kill_after * 1000f64) as u64));
            match process.wait() {
                Ok(status) => {
                    if preserve_status {
                        match status {
                            ExitStatus(stat) => stat,
                            ExitSignal(stat) => stat
                        }
                    } else {
                        124
                    }
                }
                Err(_) => {
                    if kill_after == 0f64 {
                        // XXX: this may not be right
                        return 124;
                    }
                    return_if_err!(ERR_EXIT_STATUS, process.signal(signals::signal_by_name_or_value("KILL").unwrap() as isize));
                    process.set_timeout(None);
                    return_if_err!(ERR_EXIT_STATUS, process.wait());
                    137
                }
            }
        }
    }
}
