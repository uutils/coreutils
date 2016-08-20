#![crate_name = "uu_kill"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Maciej Dziardziel <fiedzia@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE file
 * that was distributed with this source code.
 */

extern crate libc;

#[macro_use]
extern crate uucore;

use libc::{c_int, pid_t};
use std::io::{Error, Write};
use uucore::signals::ALL_SIGNALS;

static SYNTAX: &'static str = "[options] <pid> [...]"; 
static SUMMARY: &'static str = ""; 
static LONG_HELP: &'static str = ""; 

static EXIT_OK:  i32 = 0;
static EXIT_ERR: i32 = 1;

#[derive(Clone, Copy)]
pub enum Mode {
    Kill,
    Table,
    List,
}

pub fn uumain(args: Vec<String>) -> i32 {
    let (args, obs_signal) = handle_obsolete(args);
    let matches = new_coreopts!(SYNTAX, SUMMARY, LONG_HELP)
        .optopt("s", "signal", "specify the <signal> to be sent", "SIGNAL")
        .optflagopt("l", "list", "list all signal names, or convert one to a name", "LIST")
        .optflag("L", "table", "list all signal names in a nice table")
        .parse(args);

    let mode = if matches.opt_present("table") {
        Mode::Table
    } else if matches.opt_present("list") {
        Mode::List
    } else {
        Mode::Kill
    };

    match mode {
        Mode::Kill    => return kill(&matches.opt_str("signal").unwrap_or(obs_signal.unwrap_or("9".to_owned())), matches.free),
        Mode::Table   => table(),
        Mode::List    => list(matches.opt_str("list")),
    }

    0
}

fn handle_obsolete(mut args: Vec<String>) -> (Vec<String>, Option<String>) {
    let mut i = 0;
    while i < args.len() {
        // this is safe because slice is valid when it is referenced
        let slice = &args[i].clone();
        if slice.chars().next().unwrap() == '-' && slice.len() > 1 && slice.chars().nth(1).unwrap().is_digit(10) {
            let val = &slice[1..];
            match val.parse() {
                Ok(num) => {
                    if uucore::signals::is_signal(num) {
                        args.remove(i);
                        return (args, Some(val.to_owned()));
                    }
                }
                Err(_)=> break  /* getopts will error out for us */
            }
        }
        i += 1;
    }
    (args, None)
}

fn table() {
    let mut name_width = 0;
    /* Compute the maximum width of a signal name. */
    for s in &ALL_SIGNALS {
        if s.name.len() > name_width {
            name_width = s.name.len()
        }
    }

    for (idx, signal) in ALL_SIGNALS.iter().enumerate() {
        print!("{0: >#2} {1: <#8}", idx+1, signal.name);
        //TODO: obtain max signal width here

        if (idx+1) % 7 == 0 {
            println!("");
        }
    }
}

fn print_signal(signal_name_or_value: &str) {
    for signal in &ALL_SIGNALS {
        if signal.name == signal_name_or_value  || (format!("SIG{}", signal.name)) == signal_name_or_value {
            println!("{}", signal.value);
            exit!(EXIT_OK as i32)
        } else if signal_name_or_value == signal.value.to_string() {
            println!("{}", signal.name);
            exit!(EXIT_OK as i32)
        }
    }
    crash!(EXIT_ERR, "unknown signal name {}", signal_name_or_value)
}

fn print_signals() {
    let mut pos = 0;
    for (idx, signal) in ALL_SIGNALS.iter().enumerate() {
        pos += signal.name.len();
        print!("{}", signal.name);
        if idx > 0 && pos > 73 {
            println!("");
            pos = 0;
        } else {
            pos += 1;
            print!(" ");
        }
    }
}

fn list(arg: Option<String>) {
    match arg {
      Some(ref x) => print_signal(x),
      None => print_signals(),
    };
}

fn kill(signalname: &str, pids: std::vec::Vec<String>) -> i32 {
    let mut status = 0;
    let optional_signal_value = uucore::signals::signal_by_name_or_value(signalname);
    let signal_value = match optional_signal_value {
        Some(x) => x,
        None => crash!(EXIT_ERR, "unknown signal name {}", signalname)
    };
    for pid in &pids {
        match pid.parse::<usize>() {
            Ok(x) => {
                if unsafe { libc::kill(x as pid_t, signal_value as c_int) } != 0 {
                    show_error!("{}", Error::last_os_error());
                    status = 1;
                }
            },
            Err(e) => crash!(EXIT_ERR, "failed to parse argument {}: {}", pid, e)
        };
    }
    status
}
