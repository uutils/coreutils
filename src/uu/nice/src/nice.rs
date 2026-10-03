// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) getpriority setpriority nstr PRIO

use clap::{Arg, ArgAction, Command};
use std::ffi::OsString;
use std::io::{ErrorKind, Write, stdout};
#[cfg(unix)]
use std::os::unix::process::CommandExt as _;
#[cfg(windows)]
use std::os::windows::process::CommandExt as _;
use std::process;
#[cfg(windows)]
use windows_sys::Win32::System::Threading::{
    ABOVE_NORMAL_PRIORITY_CLASS, BELOW_NORMAL_PRIORITY_CLASS, GetCurrentProcess, GetPriorityClass,
    HIGH_PRIORITY_CLASS, IDLE_PRIORITY_CLASS, NORMAL_PRIORITY_CLASS, REALTIME_PRIORITY_CLASS,
};

use uucore::translate;
use uucore::{
    error::{UResult, UUsageError, set_exit_code},
    format_usage, show_error,
};

pub mod options {
    pub static ADJUSTMENT: &str = "adjustment";
    pub static COMMAND: &str = "COMMAND";
}

/// Transform legacy arguments into a standardized form.
///
/// The following are all legal argument sequences to GNU nice:
/// - "-1"
/// - "-n1"
/// - "-+1"
/// - "--1"
/// - "-n -1"
///
/// It looks initially like we could add handling for "-{i}", "--{i}"
/// and "-+{i}" for integers {i} and process them normally using clap.
/// However, the meaning of "-1", for example, changes depending on
/// its context with legacy argument parsing. clap will not prioritize
/// hyphenated values to previous arguments over matching a known
/// argument.  So "-n" "-1" in this case is picked up as two
/// arguments, not one argument with a value.
///
/// Given this context dependency, and the deep hole we end up digging
/// with clap in this case, it's much simpler to just normalize the
/// arguments to nice before clap starts work. Here, we insert a
/// prefix of "-n" onto all arguments of the form "-{i}", "--{i}" and
/// "-+{i}" which are not already preceded by "-n".
fn standardize_nice_args(mut args: impl uucore::Args) -> impl uucore::Args {
    let mut v = Vec::<OsString>::new();
    let mut saw_n = false;
    let mut saw_command = false;
    if let Some(cmd) = args.next() {
        v.push(cmd);
    }
    for s in args {
        if saw_command {
            v.push(s);
        } else if saw_n {
            let mut new_arg: OsString = "-n".into();
            new_arg.push(s);
            v.push(new_arg);
            saw_n = false;
        } else if s
            .to_str()
            .is_some_and(|s| s == "-n" || (s.len() >= "--a".len() && "--adjustment".starts_with(s)))
        {
            saw_n = true;
        } else if let Ok(s) = s.clone().into_string() {
            if let Some(stripped) = s.strip_prefix('-') {
                match stripped.parse::<i64>() {
                    Ok(ix) => {
                        let mut new_arg: OsString = "-n".into();
                        new_arg.push(ix.to_string());
                        v.push(new_arg);
                    }
                    Err(_) => {
                        v.push(s.into());
                    }
                }
            } else {
                saw_command = true;
                v.push(s.into());
            }
        } else {
            saw_command = true;
            v.push(s);
        }
    }
    if saw_n {
        v.push("-n".into());
    }
    v.into_iter()
}

/// Map a Windows priority class to the nice value Cygwin reports for it.
#[cfg(windows)]
fn niceness_from_priority_class(priority_class: u32) -> i32 {
    match priority_class {
        REALTIME_PRIORITY_CLASS => -20,
        HIGH_PRIORITY_CLASS => -16,
        ABOVE_NORMAL_PRIORITY_CLASS => -8,
        BELOW_NORMAL_PRIORITY_CLASS => 8,
        IDLE_PRIORITY_CLASS => 16,
        // NORMAL, and the 0 that `GetPriorityClass` returns when it fails.
        _ => 0,
    }
}

/// Map a nice value to a Windows priority class, like Cygwin's `setpriority`.
#[cfg(windows)]
fn priority_class_from_niceness(niceness: i32) -> u32 {
    match niceness {
        ..=-20 => REALTIME_PRIORITY_CLASS,
        -19..=-13 => HIGH_PRIORITY_CLASS,
        -12..=-5 => ABOVE_NORMAL_PRIORITY_CLASS,
        -4..=3 => NORMAL_PRIORITY_CLASS,
        4..=11 => BELOW_NORMAL_PRIORITY_CLASS,
        _ => IDLE_PRIORITY_CLASS,
    }
}

/// The priority class the current process runs at.
#[cfg(windows)]
fn get_current_priority_class() -> u32 {
    // SAFETY: `GetCurrentProcess` always returns a valid pseudo-handle.
    let process = unsafe { GetCurrentProcess() };

    // SAFETY: `process` is a valid handle for querying the priority class.
    unsafe { GetPriorityClass(process) }
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args = standardize_nice_args(args);

    let matches =
        uucore::clap_localization::handle_clap_result_with_exit_code(uu_app(), args, 125)?;

    #[cfg(not(any(unix, windows)))]
    let current_niceness = 0i32;
    #[cfg(windows)]
    let current_niceness = niceness_from_priority_class(get_current_priority_class());
    #[cfg(unix)]
    let current_niceness = rustix::process::getpriority_process(None)
        .map_err(|e| uucore::error::USimpleError::new(125, format!("getpriority: {e}")))?;

    let Some(mut cmd_iter) = matches.get_many::<String>(options::COMMAND) else {
        if matches.contains_id(options::ADJUSTMENT) {
            return Err(UUsageError::new(
                125,
                translate!("nice-error-command-required-with-adjustment"),
            ));
        }

        writeln!(stdout(), "{current_niceness}")?;
        return Ok(());
    };

    #[cfg(any(unix, windows))]
    let adjustment = match matches.get_one::<String>(options::ADJUSTMENT) {
        None => 10,
        Some(nstr) => match nstr.parse::<i32>() {
            Ok(num) => num,
            Err(e) if *e.kind() == std::num::IntErrorKind::PosOverflow => i32::MAX,
            Err(e) if *e.kind() == std::num::IntErrorKind::NegOverflow => i32::MIN,
            Err(e) => {
                return Err(uucore::error::USimpleError::new(
                    125,
                    translate!("nice-error-invalid-number", "value" => nstr, "error" => e),
                ));
            }
        },
    };

    #[cfg(any(unix, windows))]
    let new_niceness = current_niceness.saturating_add(adjustment);
    // We can't use `show_warning` because that will panic if stderr
    // isn't writable. The GNU test suite checks specifically that the
    // exit code when failing to write the advisory is 125, but Rust
    // will produce an exit code of 101 when it panics.
    #[cfg(unix)]
    if let Err(e) = rustix::process::setpriority_process(None, new_niceness) {
        let warning_msg = translate!("nice-warning-setpriority", "util_name" => "nice", "error" => uucore::error::strip_errno(&e.into()) );

        if writeln!(std::io::stderr(), "{warning_msg}").is_err() {
            set_exit_code(125);
            return Ok(());
        }
    }

    #[cfg(windows)]
    let priority_class = priority_class_from_niceness(new_niceness);

    let cmd = cmd_iter.next().unwrap();
    let args: Vec<&String> = cmd_iter.collect();
    let mut command = process::Command::new(cmd);
    command.args(args);
    #[cfg(unix)]
    let err = command.exec();
    #[cfg(windows)]
    let err = match command.creation_flags(priority_class).spawn() {
        Ok(mut child) => {
            return match child.wait() {
                Ok(status) => {
                    set_exit_code(status.code().unwrap_or(1));
                    Ok(())
                }
                Err(e) => {
                    show_error!("{cmd}: {e}");
                    set_exit_code(126);
                    Ok(())
                }
            };
        }
        Err(err) => err,
    };
    #[cfg(not(any(unix, windows)))]
    let Err(err) = command.status() else {
        return Ok(());
    };

    show_error!("{cmd}: {err}");

    let exit_code = if err.kind() == ErrorKind::NotFound {
        127
    } else {
        126
    };
    set_exit_code(exit_code);
    Ok(())
}

pub fn uu_app() -> Command {
    Command::new("nice")
        .about(translate!("nice-about"))
        .override_usage(format_usage(&translate!("nice-usage")))
        .trailing_var_arg(true)
        .infer_long_args(true)
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template("nice"))
        .arg(
            Arg::new(options::ADJUSTMENT)
                .short('n')
                .long(options::ADJUSTMENT)
                .help(translate!("nice-help-adjustment"))
                .action(ArgAction::Set)
                .overrides_with(options::ADJUSTMENT)
                .allow_hyphen_values(true),
        )
        .arg(
            Arg::new(options::COMMAND)
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::CommandName),
        )
}
