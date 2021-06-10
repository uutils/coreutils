//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Maciej Dziardziel <fiedzia@gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE file
//  * that was distributed with this source code.

// spell-checker:ignore (ToDO) signalname pids

#[macro_use]
extern crate uucore;

use clap::{crate_version, App, Arg};
use libc::{c_int, pid_t};
use std::io::Error;
use uucore::signals::ALL_SIGNALS;
use uucore::InvalidEncodingHandling;

static ABOUT: &str = "Send signal to processes or list information about signals.";

static EXIT_OK: i32 = 0;
static EXIT_ERR: i32 = 1;

pub mod options {
    pub static PIDS_OR_SIGNALS: &str = "pids_of_signals";
    pub static LIST: &str = "list";
    pub static TABLE: &str = "table";
    pub static TABLE_OLD: &str = "table_old";
    pub static SIGNAL: &str = "signal";
}

#[derive(Clone, Copy)]
pub enum Mode {
    Kill,
    Table,
    List,
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let args = args
        .collect_str(InvalidEncodingHandling::Ignore)
        .accept_any();
    let (args, obs_signal) = handle_obsolete(args);

    let usage = format!("{} [OPTIONS]... PID...", executable!());
    let matches = App::new(executable!())
        .version(crate_version!())
        .about(ABOUT)
        .usage(&usage[..])
        .arg(
            Arg::with_name(options::LIST)
                .short("l")
                .long(options::LIST)
                .help("Lists signals")
                .conflicts_with(options::TABLE)
                .conflicts_with(options::TABLE_OLD),
        )
        .arg(
            Arg::with_name(options::TABLE)
                .short("t")
                .long(options::TABLE)
                .help("Lists table of signals"),
        )
        .arg(Arg::with_name(options::TABLE_OLD).short("L").hidden(true))
        .arg(
            Arg::with_name(options::SIGNAL)
                .short("s")
                .long(options::SIGNAL)
                .help("Sends given signal")
                .takes_value(true),
        )
        .arg(
            Arg::with_name(options::PIDS_OR_SIGNALS)
                .hidden(true)
                .multiple(true),
        )
        .get_matches_from(args);

    let mode = if matches.is_present(options::TABLE) || matches.is_present(options::TABLE_OLD) {
        Mode::Table
    } else if matches.is_present(options::LIST) {
        Mode::List
    } else {
        Mode::Kill
    };

    let pids_or_signals: Vec<String> = matches
        .values_of(options::PIDS_OR_SIGNALS)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_default();

    match mode {
        Mode::Kill => {
            let sig = match (obs_signal, matches.value_of(options::SIGNAL)) {
                (Some(s), Some(_)) => s, // -s takes precedence
                (Some(s), None) => s,
                (None, Some(s)) => s.to_owned(),
                (None, None) => "TERM".to_owned(),
            };
            return kill(&sig, &pids_or_signals);
        }
        Mode::Table => table(),
        Mode::List => list(pids_or_signals.get(0).cloned()),
    }

    EXIT_OK
}

fn handle_obsolete(mut args: Vec<String>) -> (Vec<String>, Option<String>) {
    let mut i = 0;
    while i < args.len() {
        // this is safe because slice is valid when it is referenced
        let slice = &args[i].clone();
        if slice.starts_with('-') && slice.chars().nth(1).map_or(false, |c| c.is_digit(10)) {
            let val = &slice[1..];
            match val.parse() {
                Ok(num) => {
                    if uucore::signals::is_signal(num) {
                        args.remove(i);
                        return (args, Some(val.to_owned()));
                    }
                }
                Err(_) => break, /* getopts will error out for us */
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
        print!("{0: >#2} {1: <#8}", idx + 1, signal.name);
        //TODO: obtain max signal width here

        if (idx + 1) % 7 == 0 {
            println!();
        }
    }
}

fn print_signal(signal_name_or_value: &str) {
    for signal in &ALL_SIGNALS {
        if signal.name == signal_name_or_value
            || (format!("SIG{}", signal.name)) == signal_name_or_value
        {
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
            println!();
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

fn kill(signalname: &str, pids: &[String]) -> i32 {
    let mut status = 0;
    let optional_signal_value = uucore::signals::signal_by_name_or_value(signalname);
    let signal_value = match optional_signal_value {
        Some(x) => x,
        None => crash!(EXIT_ERR, "unknown signal name {}", signalname),
    };
    for pid in pids {
        match pid.parse::<usize>() {
            Ok(x) => {
                if unsafe { libc::kill(x as pid_t, signal_value as c_int) } != 0 {
                    show_error!("{}", Error::last_os_error());
                    status = 1;
                }
            }
            Err(e) => crash!(EXIT_ERR, "failed to parse argument {}: {}", pid, e),
        };
    }
    status
}
