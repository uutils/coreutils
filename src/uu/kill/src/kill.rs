// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) signalname pids killpg NOPESIG

use clap::{Arg, ArgAction, Command};
use rustix::process::{
    Pid, Signal, kill_current_process_group, kill_process, kill_process_group,
    test_kill_current_process_group, test_kill_process, test_kill_process_group,
};
use std::cmp::Ordering;
use std::io::{self, BufWriter, Write};
use uucore::display::Quotable;
use uucore::error::{FromIo, UError, UResult, USimpleError, strip_errno};
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
    let obs_signal = handle_obsolete(&mut args)?;

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
                return Err(USimpleError::new(1, translate!("kill-error-no-process-id")));
            }

            kill(sig, &pids);
        }
        Mode::Table => table()?,
        Mode::List => list(&pids_or_signals)?,
    }

    Ok(())
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

fn handle_obsolete(args: &mut Vec<String>) -> UResult<Option<usize>> {
    // Sanity check - need at least the program name and one argument
    if args.len() >= 2 {
        // Old signal can only be in the first argument position
        let slice = args[1].as_str();
        if let Some(signal) = slice.strip_prefix('-') {
            // With '-', a signal name must start with an uppercase char
            if signal.chars().next().is_some_and(char::is_lowercase) {
                return Ok(None);
            }
            // Check if it is a valid signal
            if let Some(signal_value) = signal_by_name_or_value(signal) {
                // remove the signal before return
                args.remove(1);
                return Ok(Some(signal_value));
            }
            // Not a known signal. If the argument still looks like an obsolete
            // signal specification (a number, or a multi-character uppercase
            // name), reject it like GNU instead of letting it fall through to
            // be parsed as a negative PID and silently signalled with SIGTERM.
            let first = signal.chars().next();
            let looks_like_signal = first.is_some_and(|c| c.is_ascii_digit())
                || (signal.len() > 1 && first.is_some_and(|c| c.is_ascii_uppercase()));
            if looks_like_signal {
                return Err(USimpleError::new(
                    1,
                    translate!("kill-error-invalid-signal", "signal" => signal.quote()),
                ));
            }
        }
    }
    Ok(None)
}

// Turn a stdout write failure into a user-facing error carrying the OS error
// message (e.g. "write error: No space left on device"), so a failed write to a
// full/closed stdout is reported instead of panicking inside `println!`.
fn stdout_write_error(err: io::Error) -> Box<dyn UError> {
    USimpleError::new(
        1,
        translate!("kill-error-write-error", "err" => strip_errno(&err)),
    )
}

fn table() -> UResult<()> {
    // Buffer the listing so a failed write surfaces as one clean error at flush
    // rather than the runtime's implicit-flush message on top of ours.
    let mut out = BufWriter::new(io::stdout().lock());
    for signal_value in 0..=signal_number_upper_bound() {
        if let Some(signal_name) = signal_list_name_by_value(signal_value) {
            writeln!(out, "{signal_value: >#2} {signal_name}").map_err(stdout_write_error)?;
        }
    }
    out.flush().map_err(stdout_write_error)?;
    Ok(())
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
    // Resolve the signal to the text kill would print, so the write path is the
    // same for every branch (a single buffered write + flush).
    let output = if let Some(signal_value) = signal_name_or_value
        .parse::<usize>()
        .ok()
        .and_then(normalize_list_signal_value)
    {
        // GNU kill accepts plain signal numbers, values masked to the low 8 bits,
        // and exit statuses that encode `128 + signal`.
        signal_list_name_by_value(signal_value).unwrap_or_else(|| signal_value.to_string())
    } else if let Some(signal_value) = signal_list_value_by_name_or_number(signal_name_or_value) {
        signal_value.to_string()
    } else {
        return Err(USimpleError::new(
            1,
            translate!("kill-error-invalid-signal", "signal" => signal_name_or_value.quote()),
        ));
    };

    let mut out = BufWriter::new(io::stdout().lock());
    writeln!(out, "{output}").map_err(stdout_write_error)?;
    out.flush().map_err(stdout_write_error)?;
    Ok(())
}

fn print_signals() -> UResult<()> {
    let mut out = BufWriter::new(io::stdout().lock());
    for signal_value in 0..=signal_number_upper_bound() {
        if let Some(signal_name) = signal_list_name_by_value(signal_value) {
            writeln!(out, "{signal_name}").map_err(stdout_write_error)?;
        }
    }
    out.flush().map_err(stdout_write_error)?;
    Ok(())
}

fn list(signals: &Vec<String>) -> UResult<()> {
    if signals.is_empty() {
        print_signals()?;
    } else {
        for signal in signals {
            if let Err(e) = print_signal(signal) {
                uucore::show!(e);
            }
        }
    }
    Ok(())
}

fn rustix_to_io(result: rustix::io::Result<()>) -> io::Result<()> {
    result.map_err(io::Error::from)
}

// rustix's `Signal` rejects libc-reserved realtime signals, so fall back to a
// raw `libc::kill` for any value its safe constructor doesn't recognize.
fn raw_kill(pid: i32, sig: usize) -> io::Result<()> {
    let sig = i32::try_from(sig).map_err(|_| io::Error::from_raw_os_error(libc::EINVAL))?;
    // SAFETY: plain FFI call; `kill` has no memory-safety preconditions.
    if unsafe { libc::kill(pid as libc::pid_t, sig) } == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
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

fn kill(sig: usize, pids: &[i32]) {
    // Standard named signals use rustix's typed API; anything its safe
    // constructor doesn't recognize (realtime/reserved) falls back to libc.
    let named = (sig != 0)
        .then(|| i32::try_from(sig).ok().and_then(Signal::from_named_raw))
        .flatten();
    for &pid in pids {
        let result = match pid.cmp(&0) {
            Ordering::Equal => match named {
                _ if sig == 0 => rustix_to_io(test_kill_current_process_group()),
                Some(s) => rustix_to_io(kill_current_process_group(s)),
                None => raw_kill(0, sig),
            },
            Ordering::Greater => {
                let pid = Pid::from_raw(pid).expect("pid > 0 guaranteed by Ordering::Greater");
                match named {
                    _ if sig == 0 => rustix_to_io(test_kill_process(pid)),
                    Some(s) => rustix_to_io(kill_process(pid, s)),
                    None => raw_kill(pid.as_raw_nonzero().get(), sig),
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
                match named {
                    _ if sig == 0 => rustix_to_io(test_kill_process_group(pid)),
                    Some(s) => rustix_to_io(kill_process_group(pid, s)),
                    None => raw_kill(-pid.as_raw_nonzero().get(), sig),
                }
            }
        };
        if let Err(e) = result {
            show!(e.map_err_context(|| translate!("kill-error-sending-signal", "pid" => pid)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::handle_obsolete;

    fn args(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn test_handle_obsolete() {
        // A valid obsolete signal (name or number) is consumed; SIGKILL is 9
        // on every supported platform.
        let mut a = args(&["kill", "-KILL", "123"]);
        assert_eq!(handle_obsolete(&mut a).unwrap(), Some(9));
        assert_eq!(a, args(&["kill", "123"]));

        let mut a = args(&["kill", "-9", "123"]);
        assert_eq!(handle_obsolete(&mut a).unwrap(), Some(9));
        assert_eq!(a, args(&["kill", "123"]));

        // Things that look like a signal but aren't must error, not fall
        // through to be read as a negative PID and signalled with SIGTERM.
        assert!(handle_obsolete(&mut args(&["kill", "-65", "123"])).is_err());
        assert!(handle_obsolete(&mut args(&["kill", "-NOPESIG", "123"])).is_err());

        // A lowercase leading char is never an obsolete signal; leave args as-is.
        let mut a = args(&["kill", "-foo", "123"]);
        assert_eq!(handle_obsolete(&mut a).unwrap(), None);
        assert_eq!(a, args(&["kill", "-foo", "123"]));

        // Not enough arguments to carry an obsolete signal.
        assert_eq!(handle_obsolete(&mut args(&["kill"])).unwrap(), None);
    }
}
