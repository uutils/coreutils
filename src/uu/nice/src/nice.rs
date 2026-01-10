// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) getpriority setpriority nstr PRIO

use clap::{Arg, ArgAction, Command};
use libc::PRIO_PROCESS;
use std::ffi::OsString;
use std::io::{Error, ErrorKind, Write};
use std::os::unix::process::CommandExt;
use std::process;

use uucore::translate;
use uucore::{
    error::{UResult, USimpleError, UUsageError, set_exit_code},
    format_usage, show_error,
};

pub mod options {
    pub static ADJUSTMENT: &str = "adjustment";
    pub static COMMAND: &str = "COMMAND";
}

fn is_prefix_of(maybe_prefix: &str, target: &str, min_match: usize) -> bool {
    if maybe_prefix.len() < min_match || maybe_prefix.len() > target.len() {
        return false;
    }

    &target[0..maybe_prefix.len()] == maybe_prefix
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
        } else if s.to_str() == Some("-n")
            || s.to_str()
                .is_some_and(|s| is_prefix_of(s, "--adjustment", "--a".len()))
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

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args = standardize_nice_args(args);

    let matches =
        uucore::clap_localization::handle_clap_result_with_exit_code(uu_app(), args, 125)?;

    nix::errno::Errno::clear();
    let mut niceness = unsafe { libc::getpriority(PRIO_PROCESS, 0) };
    if Error::last_os_error().raw_os_error().unwrap() != 0 {
        return Err(USimpleError::new(
            125,
            format!("getpriority: {}", Error::last_os_error()),
        ));
    }

    let adjustment = match matches.get_one::<String>(options::ADJUSTMENT) {
        Some(nstr) => {
            if !matches.contains_id(options::COMMAND) {
                return Err(UUsageError::new(
                    125,
                    translate!("nice-error-command-required-with-adjustment"),
                ));
            }
            match nstr.parse::<i32>() {
                Ok(num) => num,
                Err(e) => {
                    return Err(USimpleError::new(
                        125,
                        translate!("nice-error-invalid-number", "value" => nstr.clone(), "error" => e),
                    ));
                }
            }
        }
        None => {
            if !matches.contains_id(options::COMMAND) {
                println!("{niceness}");
                return Ok(());
            }
            10_i32
        }
    };

    niceness += adjustment;
    // We can't use `show_warning` because that will panic if stderr
    // isn't writable. The GNU test suite checks specifically that the
    // exit code when failing to write the advisory is 125, but Rust
    // will produce an exit code of 101 when it panics.
    if unsafe { libc::setpriority(PRIO_PROCESS, 0, niceness) } == -1 {
        let warning_msg = translate!("nice-warning-setpriority", "util_name" => uucore::util_name(), "error" => Error::last_os_error());

        if write!(std::io::stderr(), "{warning_msg}").is_err() {
            set_exit_code(125);
            return Ok(());
        }
    }

    let mut cmd_iter = matches.get_many::<String>(options::COMMAND).unwrap();
    let cmd = cmd_iter.next().unwrap();
    let args: Vec<&String> = cmd_iter.collect();

    let err = process::Command::new(cmd).args(args).exec();

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
    Command::new(uucore::util_name())
        .about(translate!("nice-about"))
        .override_usage(format_usage(&translate!("nice-usage")))
        .trailing_var_arg(true)
        .infer_long_args(true)
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
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
