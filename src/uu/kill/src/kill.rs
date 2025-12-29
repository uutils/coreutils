// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) signalname pids killpg

use clap::{Arg, ArgAction, Command};
use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;
use std::io::Error;
use uucore::display::Quotable;
use uucore::error::{FromIo, UResult, USimpleError};
use uucore::translate;

use uucore::signals::{ALL_SIGNALS, signal_by_name_or_value, signal_name_by_value};
use uucore::{format_usage, show};

// When the -l option is selected, the program displays the type of signal related to a certain
// value or string. In case of a value, the program should control the lower 8 bits, but there is
// a particular case in which if the value is in range [128, 159], it is translated to a signal
const OFFSET: usize = 128;

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

    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;

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

            let sig_name = signal_name_by_value(sig);
            // Signal does not support converting from EXIT
            // Instead, nix::signal::kill expects Option::None to properly handle EXIT
            let sig: Option<Signal> = if sig_name.is_some_and(|name| name == "EXIT") {
                None
            } else {
                let sig = (sig as i32)
                    .try_into()
                    .map_err(|e| Error::from_raw_os_error(e as i32))?;
                Some(sig)
            };

            let pids = parse_pids(&pids_or_signals)?;
            if pids.is_empty() {
                Err(USimpleError::new(1, translate!("kill-error-no-process-id")))
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
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .about(translate!("kill-about"))
        .override_usage(format_usage(&translate!("kill-usage")))
        .infer_long_args(true)
        .allow_negative_numbers(true)
        .arg(
            Arg::new(options::LIST)
                .short('l')
                .long(options::LIST)
                .help(translate!("kill-help-list"))
                .conflicts_with(options::TABLE)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::TABLE)
                .short('t')
                .short_alias('L')
                .long(options::TABLE)
                .help(translate!("kill-help-table"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SIGNAL)
                .short('s')
                .short_alias('n') // For bash compatibility, like in GNU coreutils
                .long(options::SIGNAL)
                .value_name("signal")
                .help(translate!("kill-help-signal"))
                .conflicts_with_all([options::LIST, options::TABLE]),
        )
        .arg(
            Arg::new(options::PIDS_OR_SIGNALS)
                .hide(true)
                .action(ArgAction::Append),
        )
}

fn handle_obsolete(args: &mut Vec<String>) -> Option<usize> {
    // Sanity check - need at least the program name and one argument
    if args.len() >= 2 {
        // Old signal can only be in the first argument position
        let slice = args[1].as_str();
        if let Some(signal) = slice.strip_prefix('-') {
            // With '-', a signal name must start with an uppercase char
            if signal.chars().next().is_some_and(|c| c.is_lowercase()) {
                return None;
            }
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
    for (idx, signal) in ALL_SIGNALS.iter().enumerate() {
        println!("{idx: >#2} {signal}");
    }
}

fn print_signal(signal_name_or_value: &str) -> UResult<()> {
    // Closure used to track the last 8 bits of the signal value
    // when the -l option is passed only the lower 8 bits are important
    // or the value is in range [128, 159]
    // Example: kill -l 143 => TERM because 143 = 15 + 128
    // Example: kill -l 2304 => EXIT
    let lower_8_bits = |x: usize| x & 0xff;
    let option_num_parse = signal_name_or_value.parse::<usize>().ok();

    for (value, &signal) in ALL_SIGNALS.iter().enumerate() {
        if signal.eq_ignore_ascii_case(signal_name_or_value)
            || format!("SIG{signal}").eq_ignore_ascii_case(signal_name_or_value)
        {
            println!("{value}");
            return Ok(());
        } else if signal_name_or_value == value.to_string()
            || option_num_parse.is_some_and(|signal_value| lower_8_bits(signal_value) == value)
            || option_num_parse.is_some_and(|signal_value| signal_value == value + OFFSET)
        {
            println!("{signal}");
            return Ok(());
        }
    }
    Err(USimpleError::new(
        1,
        translate!("kill-error-invalid-signal", "signal" => signal_name_or_value.quote()),
    ))
}

fn print_signals() {
    for signal in ALL_SIGNALS {
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
    let optional_signal_value = signal_by_name_or_value(signal_name);
    match optional_signal_value {
        Some(x) => Ok(x),
        None => Err(USimpleError::new(
            1,
            translate!("kill-error-invalid-signal", "signal" => signal_name.quote()),
        )),
    }
}

fn parse_pids(pids: &[String]) -> UResult<Vec<i32>> {
    pids.iter()
        .map(|x| {
            x.parse::<i32>().map_err(|e| {
                USimpleError::new(
                    1,
                    translate!("kill-error-parse-argument", "argument" => x.quote(), "error" => e),
                )
            })
        })
        .collect()
}

fn kill(sig: Option<Signal>, pids: &[i32]) {
    for &pid in pids {
        if let Err(e) = signal::kill(Pid::from_raw(pid), sig) {
            show!(
                Error::from_raw_os_error(e as i32)
                    .map_err_context(|| { translate!("kill-error-sending-signal", "pid" => pid) })
            );
        }
    }
}
