#![crate_id(name="kill", vers="0.0.1", author="Maciej Dziardziel")]
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

#[phase(syntax, link)] extern crate log;

use std::os;
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

mod signals;

#[path = "../common/util.rs"]
mod util;

static NAME: &'static str = "kill";
static VERSION:  &'static str = "0.0.1";

static EXIT_OK:  i32 = 0;
static EXIT_ERR: i32 = 1;

pub enum Mode {
    Kill,
    Table,
    List,
    Help,
    Version,
}

#[allow(dead_code)]
fn main() { uumain(os::args()); }

pub fn uumain(args: Vec<String>) {

    let opts = [
        optflag("h", "help", "display this help and exit"),
        optflag("V", "version", "output version information and exit"),
        optopt("s", "signal", "specify the <signal> to be sent", "SIGNAL"),
        optflagopt("l", "list", "list all signal names, or convert one to a name", "LIST"),
        optflag("L", "table", "list all signal names in a nice table"),
    ];

    let usage = usage("[options] <pid> [...]", opts);


    let matches = match getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(e) => {
            show_error!(EXIT_ERR, "{}\n{}", e.to_err_msg(),  get_help_text(NAME, usage.as_slice()));
            return
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
        Kill    => kill(matches.opt_str("signal").unwrap_or("9".to_string()).as_slice(), matches.free),
        Table   => table(),
        List    => list(matches.opt_str("list")),
        Help    => help(NAME, usage.as_slice()),
        Version => version(),
    }
}

fn version() {
    println!("{} {}", NAME, VERSION);
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
            exit!(EXIT_OK)
        } else if signal_name_or_value == signal.value.to_str().as_slice() {
            println!("{}", signal.name);
            exit!(EXIT_OK)
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

fn signal_by_name_or_value(signal_name_or_value: &str) -> Option<uint> {
    if signal_name_or_value == "0" {
        return Some(0);
    }
    for signal in ALL_SIGNALS.iter() {
        let long_name = format!("SIG{}", signal.name);
        if signal.name == signal_name_or_value  || (signal_name_or_value == signal.value.to_str().as_slice()) || (long_name.as_slice() == signal_name_or_value) {
            return Some(signal.value);
        }
    }
    return None;
}

fn kill(signalname: &str, pids: std::vec::Vec<String>) {
    let optional_signal_value = signal_by_name_or_value(signalname);
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
                  Err(_) => ()
                };
            },
            None => crash!(EXIT_ERR, "failed to parse argument {}", signalname)
        };
    }
}
