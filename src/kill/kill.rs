#![crate_name = "kill"]
#![feature(macro_rules)]
#![feature(phase)]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Maciej Dziardziel <fiedzia@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE file
 * that was distributed with this source code.
 */


extern crate getopts;
extern crate libc;
extern crate collections;
extern crate serialize;

#[phase(plugin, link)] extern crate log;

use std::from_str::from_str;
use std::io::process::Process;

use getopts::{
    getopts,
    optopt,
    optflag,
    optflagopt,
    usage,
};

use signals::ALL_SIGNALS;

#[path = "../common/util.rs"]
mod util;

#[path = "../common/signals.rs"]
mod signals;

static NAME: &'static str = "kill";
static VERSION:  &'static str = "0.0.1";

static EXIT_OK:  int = 0;
static EXIT_ERR: int = 1;

pub enum Mode {
    Kill,
    Table,
    List,
    Help,
    Version,
}

pub fn uumain(args: Vec<String>) -> int {
    let opts = [
        optflag("h", "help", "display this help and exit"),
        optflag("V", "version", "output version information and exit"),
        optopt("s", "signal", "specify the <signal> to be sent", "SIGNAL"),
        optflagopt("l", "list", "list all signal names, or convert one to a name", "LIST"),
        optflag("L", "table", "list all signal names in a nice table"),
    ];

    let usage = usage("[options] <pid> [...]", opts);

    let (args, obs_signal) = handle_obsolete(args);

    let matches = match getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(e) => {
            show_error!("{}\n{}", e,  get_help_text(NAME, usage.as_slice()));
            return EXIT_ERR;
        },
    };

    let mode = if matches.opt_present("version") {
        Version
    } else if matches.opt_present("help") {
        Help
    } else if matches.opt_present("table") {
        Table
    } else if matches.opt_present("list") {
        List
    } else {
        Kill
    };

    match mode {
        Kill    => return kill(matches.opt_str("signal").unwrap_or(obs_signal.unwrap_or("9".to_string())).as_slice(), matches.free),
        Table   => table(),
        List    => list(matches.opt_str("list")),
        Help    => help(NAME, usage.as_slice()),
        Version => version(),
    }

    0
}

fn version() {
    println!("{} {}", NAME, VERSION);
}

fn handle_obsolete(mut args: Vec<String>) -> (Vec<String>, Option<String>) {
    let mut i = 0;
    while i < args.len() {
        // this is safe because slice is valid when it is referenced
        let slice: &str = unsafe { std::mem::transmute(args[i].as_slice()) };
        if slice.char_at(0) == '-' && slice.len() > 1 && slice.char_at(1).is_digit() {
            let val = slice.slice_from(1);
            match from_str(val) {
                Some(num) => {
                    if signals::is_signal(num) {
                        args.remove(i);
                        return (args, Some(val.to_string()));
                    }
                }
                None => break  /* getopts will error out for us */
            }
        }
        i += 1;
    }
    (args, None)
}

fn table() {
    let mut name_width = 0;
    /* Compute the maximum width of a signal name. */
    for s in ALL_SIGNALS.iter() {
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
    for signal in ALL_SIGNALS.iter() {
        if signal.name == signal_name_or_value  || (format!("SIG{}", signal.name).as_slice()) == signal_name_or_value {
            println!("{}", signal.value)
            exit!(EXIT_OK as i32)
        } else if signal_name_or_value == signal.value.to_string().as_slice() {
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
      Some(x) => print_signal(x.as_slice()),
      None => print_signals(),
    };
}

fn get_help_text(progname: &str, usage: &str) -> String {
    format!("Usage: \n {0} {1}", progname, usage)
}

fn help(progname: &str, usage: &str) {
    println!("{}", get_help_text(progname, usage));
}

fn kill(signalname: &str, pids: std::vec::Vec<String>) -> int {
    let mut status = 0;
    let optional_signal_value = signals::signal_by_name_or_value(signalname);
    let signal_value = match optional_signal_value {
        Some(x) => x,
        None => crash!(EXIT_ERR, "unknown signal name {}", signalname)
    };
    for pid in pids.iter() {
        match from_str::<i32>(pid.as_slice()) {
            Some(x) => {
                let result = Process::kill(x, signal_value as int);
                match result {
                    Ok(_) => (),
                    Err(f) => {
                        show_error!("{}", f);
                        status = 1;
                    }
                };
            },
            None => crash!(EXIT_ERR, "failed to parse argument {}", pid)
        };
    }
    status
}
