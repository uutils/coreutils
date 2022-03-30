//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Maciej Dziardziel <fiedzia@gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE file
//  * that was distributed with this source code.

// spell-checker:ignore (ToDO) signalname pids

#[macro_use]
extern crate uucore;

use clap::{crate_version, Arg, Command};
use libc::{c_int, pid_t};
use std::io::Error;
use uucore::display::Quotable;
use uucore::error::{UResult, USimpleError};
use uucore::signals::{signal_by_name_or_value, ALL_SIGNALS};
use uucore::{format_usage, InvalidEncodingHandling};

static ABOUT: &str = "Send signal to processes or list information about signals.";
const USAGE: &str = "{} [OPTIONS]... PID...";

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

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let mut args = args
        .collect_str(InvalidEncodingHandling::Ignore)
        .accept_any();
    let obs_signal = handle_obsolete(&mut args);

    let matches = uu_app().get_matches_from(args);

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
            let sig = if let Some(signal) = obs_signal {
                signal
            } else if let Some(signal) = matches.value_of(options::SIGNAL) {
                parse_signal_value(signal)?
            } else {
                15_usize //SIGTERM
            };
            let pids = parse_pids(&pids_or_signals)?;
            kill(sig, &pids)
        }
        Mode::Table => {
            table();
            Ok(())
        }
        Mode::List => list(pids_or_signals.get(0)),
    }
}

pub fn uu_app<'a>() -> Command<'a> {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::LIST)
                .short('l')
                .long(options::LIST)
                .help("Lists signals")
                .conflicts_with(options::TABLE)
                .conflicts_with(options::TABLE_OLD),
        )
        .arg(
            Arg::new(options::TABLE)
                .short('t')
                .long(options::TABLE)
                .help("Lists table of signals"),
        )
        .arg(Arg::new(options::TABLE_OLD).short('L').hide(true))
        .arg(
            Arg::new(options::SIGNAL)
                .short('s')
                .long(options::SIGNAL)
                .help("Sends given signal")
                .takes_value(true),
        )
        .arg(
            Arg::new(options::PIDS_OR_SIGNALS)
                .hide(true)
                .multiple_occurrences(true),
        )
}

fn handle_obsolete(args: &mut Vec<String>) -> Option<usize> {
    // Sanity check
    if args.len() > 2 {
        // Old signal can only be in the first argument position
        let slice = args[1].as_str();
        if let Some(signal) = slice.strip_prefix('-') {
            // Check if it is a valid signal
            let opt_signal = signal_by_name_or_value(signal);
            if opt_signal.is_some() {
                // remove the signal before return
                args.remove(1);
                return opt_signal;
            }
        }
    }
    None
}

fn table() {
    let name_width = ALL_SIGNALS.iter().map(|n| n.len()).max().unwrap();

    for (idx, signal) in ALL_SIGNALS.iter().enumerate() {
        print!("{0: >#2} {1: <#2$}", idx, signal, name_width + 2);
        if (idx + 1) % 7 == 0 {
            println!();
        }
    }
    println!();
}

fn print_signal(signal_name_or_value: &str) -> UResult<()> {
    for (value, &signal) in ALL_SIGNALS.iter().enumerate() {
        if signal == signal_name_or_value || (format!("SIG{}", signal)) == signal_name_or_value {
            println!("{}", value);
            return Ok(());
        } else if signal_name_or_value == value.to_string() {
            println!("{}", signal);
            return Ok(());
        }
    }
    Err(USimpleError::new(
        1,
        format!("unknown signal name {}", signal_name_or_value.quote()),
    ))
}

fn print_signals() {
    for (idx, signal) in ALL_SIGNALS.iter().enumerate() {
        if idx > 0 {
            print!(" ");
        }
        print!("{}", signal);
    }
    println!();
}

fn list(arg: Option<&String>) -> UResult<()> {
    match arg {
        Some(x) => print_signal(x),
        None => {
            print_signals();
            Ok(())
        }
    }
}

fn parse_signal_value(signal_name: &str) -> UResult<usize> {
    let optional_signal_value = signal_by_name_or_value(signal_name);
    match optional_signal_value {
        Some(x) => Ok(x),
        None => Err(USimpleError::new(
            1,
            format!("unknown signal name {}", signal_name.quote()),
        )),
    }
}

fn parse_pids(pids: &[String]) -> UResult<Vec<usize>> {
    pids.iter()
        .map(|x| {
            x.parse::<usize>().map_err(|e| {
                USimpleError::new(1, format!("failed to parse argument {}: {}", x.quote(), e))
            })
        })
        .collect()
}

fn kill(signal_value: usize, pids: &[usize]) -> UResult<()> {
    for &pid in pids {
        if unsafe { libc::kill(pid as pid_t, signal_value as c_int) } != 0 {
            show!(USimpleError::new(1, format!("{}", Error::last_os_error())));
        }
    }
    Ok(())
}
