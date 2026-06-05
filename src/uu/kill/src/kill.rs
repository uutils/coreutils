// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) signalname pids killpg　NSIG

use clap::{Arg, ArgAction, Command};
use rustix::process::{
    Pid, Signal, kill_current_process_group, kill_process, kill_process_group,
    test_kill_current_process_group, test_kill_process, test_kill_process_group,
};
use std::cmp::Ordering;
use std::io;
use uucore::display::Quotable;
use uucore::error::{FromIo, UResult, USimpleError};
use uucore::translate;

use uucore::signals::{
    signal_by_name_or_value, signal_list_name_by_value, signal_list_value_by_name_or_number,
    signal_number_upper_bound,
};
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
    Command::new("kill")
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template("kill"))
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
            if signal.chars().next().is_some_and(char::is_lowercase) {
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
    for signal_value in 0..=signal_number_upper_bound() {
        if let Some(signal_name) = signal_list_name_by_value(signal_value) {
            println!("{signal_value: >#2} {signal_name}");
        }
    }
}

fn normalize_list_signal_value(signal_value: usize) -> Option<usize> {
    // `kill -l` also accepts wait-status-like values and decodes the signal
    // number from the low 8 bits.
    let lower_8_bits = signal_value & 0xff;
    if lower_8_bits <= signal_number_upper_bound() {
        return Some(lower_8_bits);
    }

    signal_value
        .checked_sub(OFFSET)
        .filter(|value| *value <= signal_number_upper_bound())
}

fn print_signal(signal_name_or_value: &str) -> UResult<()> {
    if let Ok(signal_value) = signal_name_or_value.parse::<usize>() {
        // GNU kill accepts plain signal numbers, values masked to the low 8 bits,
        // and exit statuses that encode `128 + signal`.
        if let Some(signal_value) = normalize_list_signal_value(signal_value) {
            println!(
                "{}",
                signal_list_name_by_value(signal_value).unwrap_or_else(|| signal_value.to_string())
            );
            return Ok(());
        }
    }

    if let Some(signal_value) = signal_list_value_by_name_or_number(signal_name_or_value) {
        println!("{signal_value}");
        return Ok(());
    }

    Err(USimpleError::new(
        1,
        translate!("kill-error-invalid-signal", "signal" => signal_name_or_value.quote()),
    ))
}

fn print_signals() {
    for signal_value in 0..=signal_number_upper_bound() {
        if let Some(signal_name) = signal_list_name_by_value(signal_value) {
            println!("{signal_name}");
        }
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

/// Convert a validated non-realtime signal number to a rustix [`Signal`].
///
/// # Safety (justification)
///
/// The caller must guarantee `sig > 0`. In this module that invariant is
/// upheld by the control flow in [`kill()`], which routes `sig == 0` to the
/// `test_kill_*` functions before reaching this helper.
///
/// Signal validity is ensured by [`signal_by_name_or_value`], which accepts:
/// named and numeric signals in the supported platform range. Realtime signals
/// are handled by [`raw_kill`] before this helper is called, because rustix
/// does not permit libc-reserved realtime [`Signal`] values to be used for
/// sending signals.
fn sig_from_usize(sig: usize) -> Signal {
    debug_assert!(
        sig > 0,
        "signal 0 must be handled before calling this function"
    );
    debug_assert!(!is_realtime_signal(sig));
    // SAFETY: See function-level safety comment above.
    unsafe { Signal::from_raw_unchecked(sig as i32) }
}

fn rustix_to_io(result: rustix::io::Result<()>) -> io::Result<()> {
    result.map_err(io::Error::from)
}

fn raw_kill(pid: i32, sig: usize) -> io::Result<()> {
    let sig = i32::try_from(sig).map_err(|_| io::Error::from_raw_os_error(libc::EINVAL))?;
    if unsafe { libc::kill(pid as libc::pid_t, sig) } == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn is_realtime_signal(sig: usize) -> bool {
    let Ok(sig) = i32::try_from(sig) else {
        return false;
    };
    (libc::SIGRTMIN()..=libc::SIGRTMAX()).contains(&sig)
}

#[cfg(not(any(target_os = "linux", target_os = "android")))]
fn is_realtime_signal(_: usize) -> bool {
    false
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

fn kill(sig: usize, pids: &[i32]) {
    for &pid in pids {
        let result = match pid.cmp(&0) {
            Ordering::Equal if sig == 0 => rustix_to_io(test_kill_current_process_group()),
            Ordering::Equal if is_realtime_signal(sig) => raw_kill(0, sig),
            Ordering::Equal => rustix_to_io(kill_current_process_group(sig_from_usize(sig))),
            Ordering::Greater => {
                let pid = Pid::from_raw(pid).expect("pid > 0 guaranteed by Ordering::Greater");
                if sig == 0 {
                    rustix_to_io(test_kill_process(pid))
                } else if is_realtime_signal(sig) {
                    raw_kill(pid.as_raw_nonzero().get(), sig)
                } else {
                    rustix_to_io(kill_process(pid, sig_from_usize(sig)))
                }
            }
            Ordering::Less => {
                let Some(abs_pid) = pid.checked_neg() else {
                    show!(USimpleError::new(
                        1,
                        translate!("kill-error-sending-signal", "pid" => pid),
                    ));
                    continue;
                };
                let pid =
                    Pid::from_raw(abs_pid).expect("abs_pid > 0 since pid < 0 and pid != i32::MIN");
                if sig == 0 {
                    rustix_to_io(test_kill_process_group(pid))
                } else if is_realtime_signal(sig) {
                    raw_kill(-pid.as_raw_nonzero().get(), sig)
                } else {
                    rustix_to_io(kill_process_group(pid, sig_from_usize(sig)))
                }
            }
        };
        if let Err(e) = result {
            show!(e.map_err_context(|| translate!("kill-error-sending-signal", "pid" => pid)));
        }
    }
}
