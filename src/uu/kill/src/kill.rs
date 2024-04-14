// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) signalname pids killpg

use clap::{crate_version, Arg, ArgAction, Command};
use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;
use std::io::Error;
use uucore::display::Quotable;
use uucore::error::{FromIo, UResult, USimpleError};
use uucore::signals::{signal_by_name_or_value, ALL_SIGNALS};
use uucore::{format_usage, help_about, help_usage, show};

static ABOUT: &str = help_about!("kill.md");
const USAGE: &str = help_usage!("kill.md");

pub mod options {
    pub static PIDS_OR_SIGNALS: &str = "pids_or_signals";
    pub static LIST: &str = "list";
    pub static TABLE: &str = "table";
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
    let mut args = args.collect_ignore();
    let obs_signal = handle_obsolete(&mut args);

    let matches = uu_app().try_get_matches_from(args)?;

    let mode = if matches.get_flag(options::TABLE) {
        Mode::Table
    } else if matches.get_flag(options::LIST) {
        Mode::List
    } else {
        Mode::Kill
    };

    let pids_or_signals: Vec<String> = matches
        .get_many::<String>(options::PIDS_OR_SIGNALS)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_default();

    match mode {
        Mode::Kill => {
            let sig = if let Some(signal) = obs_signal {
                signal
            } else if let Some(signal) = matches.get_one::<String>(options::SIGNAL) {
                parse_signal_value(signal)?
            } else {
                15_usize //SIGTERM
            };
            let sig: Signal = (sig as i32)
                .try_into()
                .map_err(|e| std::io::Error::from_raw_os_error(e as i32))?;
            let pids = parse_pids(&pids_or_signals)?;
            if pids.is_empty() {
                Err(USimpleError::new(
                    1,
                    "no process ID specified\n\
                     Try --help for more information.",
                ))
            } else {
                kill(sig, &pids);
                Ok(())
            }
        }
        Mode::Table => {
            table();
            Ok(())
        }
        Mode::List => {
            list(&pids_or_signals);
            Ok(())
        }
    }
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .allow_negative_numbers(true)
        .arg(
            Arg::new(options::LIST)
                .short('l')
                .long(options::LIST)
                .help("Lists signals")
                .conflicts_with(options::TABLE)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::TABLE)
                .short('t')
                .short_alias('L')
                .long(options::TABLE)
                .help("Lists table of signals")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SIGNAL)
                .short('s')
                .long(options::SIGNAL)
                .value_name("signal")
                .help("Sends given signal instead of SIGTERM"),
        )
        .arg(
            Arg::new(options::PIDS_OR_SIGNALS)
                .hide(true)
                .action(ArgAction::Append),
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
        if signal == signal_name_or_value || (format!("SIG{signal}")) == signal_name_or_value {
            println!("{value}");
            return Ok(());
        } else if signal_name_or_value == value.to_string() {
            println!("{signal}");
            return Ok(());
        }
    }
    Err(USimpleError::new(
        1,
        format!("unknown signal name {}", signal_name_or_value.quote()),
    ))
}

fn print_signals() {
    // GNU kill doesn't list the EXIT signal with --list, so we ignore it, too
    for signal in ALL_SIGNALS.iter().filter(|x| **x != "EXIT") {
        println!("{signal}");
    }
}

fn list(signals: &Vec<String>) {
    if signals.is_empty() {
        print_signals();
    } else {
        for signal in signals {
            if let Err(e) = print_signal(signal) {
                uucore::show!(e)
            }
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

fn parse_pids(pids: &[String]) -> UResult<Vec<i32>> {
    pids.iter()
        .map(|x| {
            x.parse::<i32>().map_err(|e| {
                USimpleError::new(1, format!("failed to parse argument {}: {}", x.quote(), e))
            })
        })
        .collect()
}

fn kill(sig: Signal, pids: &[i32]) {
    for &pid in pids {
        if let Err(e) = signal::kill(Pid::from_raw(pid), sig) {
            show!(Error::from_raw_os_error(e as i32)
                .map_err_context(|| format!("sending signal to {pid} failed")));
        }
    }
}
