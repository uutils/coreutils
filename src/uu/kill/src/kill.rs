// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) signalname pids killpg

use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;
use std::io::Error;
use uucore::display::Quotable;
use uucore::error::{FromIo, UResult, USimpleError};
use uucore::show;
use uucore::signals::{signal_by_name_or_value, signal_name_by_value, ALL_SIGNALS};

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

    let matches = crate::uu_app().try_get_matches_from(args)?;

    let mode = if matches.get_flag(crate::options::TABLE) {
        Mode::Table
    } else if matches.get_flag(crate::options::LIST) {
        Mode::List
    } else {
        Mode::Kill
    };

    let pids_or_signals: Vec<String> = matches
        .get_many::<String>(crate::options::PIDS_OR_SIGNALS)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_default();

    match mode {
        Mode::Kill => {
            let sig = if let Some(signal) = obs_signal {
                signal
            } else if let Some(signal) = matches.get_one::<String>(crate::options::SIGNAL) {
                parse_signal_value(signal)?
            } else {
                15_usize //SIGTERM
            };

            let sig_name = signal_name_by_value(sig);
            // Signal does not support converting from EXIT
            // Instead, nix::signal::kill expects Option::None to properly handle EXIT
            let sig: Option<Signal> = if sig_name.is_some_and(|name| name == "EXIT") {
                None
            } else {
                let sig = (sig as i32)
                    .try_into()
                    .map_err(|e| std::io::Error::from_raw_os_error(e as i32))?;
                Some(sig)
            };

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
    // GNU kill doesn't list the EXIT signal with --table, so we ignore it, too
    for (idx, signal) in ALL_SIGNALS
        .iter()
        .enumerate()
        .filter(|(_, s)| **s != "EXIT")
    {
        println!("{0: >#2} {1}", idx, signal);
    }
}

fn print_signal(signal_name_or_value: &str) -> UResult<()> {
    for (value, &signal) in ALL_SIGNALS.iter().enumerate() {
        if signal.eq_ignore_ascii_case(signal_name_or_value)
            || format!("SIG{signal}").eq_ignore_ascii_case(signal_name_or_value)
        {
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
                uucore::show!(e);
            }
        }
    }
}

fn parse_signal_value(signal_name: &str) -> UResult<usize> {
    let signal_name_upcase = signal_name.to_uppercase();
    let optional_signal_value = signal_by_name_or_value(&signal_name_upcase);
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

fn kill(sig: Option<Signal>, pids: &[i32]) {
    for &pid in pids {
        if let Err(e) = signal::kill(Pid::from_raw(pid), sig) {
            show!(Error::from_raw_os_error(e as i32)
                .map_err_context(|| format!("sending signal to {pid} failed")));
        }
    }
}
