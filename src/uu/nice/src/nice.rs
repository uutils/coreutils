// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) getpriority execvp setpriority nstr PRIO cstrs ENOENT

use libc::{c_char, c_int, execvp, PRIO_PROCESS};
use std::ffi::{CString, OsString};
use std::io::{Error, Write};
use std::ptr;

use clap::{crate_version, Arg, ArgAction, Command};
use uucore::{
    error::{set_exit_code, UClapError, UResult, USimpleError, UUsageError},
    format_usage, help_about, help_usage, show_error,
};

pub mod options {
    pub static ADJUSTMENT: &str = "adjustment";
    pub static COMMAND: &str = "COMMAND";
}

const ABOUT: &str = help_about!("nice.md");
const USAGE: &str = help_usage!("nice.md");

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
                .map(|s| is_prefix_of(s, "--adjustment", "--a".len()))
                .unwrap_or_default()
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

    let matches = uu_app().try_get_matches_from(args).with_exit_code(125)?;

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
                    "A command must be given with an adjustment.",
                ));
            }
            match nstr.parse() {
                Ok(num) => num,
                Err(e) => {
                    return Err(USimpleError::new(
                        125,
                        format!("\"{nstr}\" is not a valid number: {e}"),
                    ))
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
    if unsafe { libc::setpriority(PRIO_PROCESS, 0, niceness) } == -1
        && write!(
            std::io::stderr(),
            "{}: warning: setpriority: {}",
            uucore::util_name(),
            Error::last_os_error()
        )
        .is_err()
    {
        set_exit_code(125);
        return Ok(());
    }

    let cstrs: Vec<CString> = matches
        .get_many::<String>(options::COMMAND)
        .unwrap()
        .map(|x| CString::new(x.as_bytes()).unwrap())
        .collect();

    let mut args: Vec<*const c_char> = cstrs.iter().map(|s| s.as_ptr()).collect();
    args.push(ptr::null::<c_char>());
    unsafe {
        execvp(args[0], args.as_mut_ptr());
    }

    show_error!("execvp: {}", Error::last_os_error());
    let exit_code = if Error::last_os_error().raw_os_error().unwrap() as c_int == libc::ENOENT {
        127
    } else {
        126
    };
    set_exit_code(exit_code);
    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .trailing_var_arg(true)
        .infer_long_args(true)
        .version(crate_version!())
        .arg(
            Arg::new(options::ADJUSTMENT)
                .short('n')
                .long(options::ADJUSTMENT)
                .help("add N to the niceness (default is 10)")
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
