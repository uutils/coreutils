//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Alex Lyon <arcterus@mail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) getpriority execvp setpriority nstr PRIO cstrs ENOENT

#[macro_use]
extern crate uucore;

use libc::{c_char, c_int, execvp, PRIO_PROCESS};
use std::ffi::CString;
use std::io::Error;
use std::ptr;

use clap::{crate_version, Arg, Command};
use uucore::{
    error::{set_exit_code, UResult, USimpleError, UUsageError},
    format_usage,
};

pub mod options {
    pub static ADJUSTMENT: &str = "adjustment";
    pub static COMMAND: &str = "COMMAND";
}

const ABOUT: &str = "\
    Run COMMAND with an adjusted niceness, which affects process scheduling. \
    With no COMMAND, print the current niceness.  Niceness values range from at \
    least -20 (most favorable to the process) to 19 (least favorable to the \
    process).";
const USAGE: &str = "{} [OPTIONS] [COMMAND [ARGS]]";

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().get_matches_from(args);

    let mut niceness = unsafe {
        nix::errno::Errno::clear();
        libc::getpriority(PRIO_PROCESS, 0)
    };
    if Error::last_os_error().raw_os_error().unwrap() != 0 {
        return Err(USimpleError::new(
            125,
            format!("getpriority: {}", Error::last_os_error()),
        ));
    }

    let adjustment = match matches.value_of(options::ADJUSTMENT) {
        Some(nstr) => {
            if !matches.is_present(options::COMMAND) {
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
                        format!("\"{}\" is not a valid number: {}", nstr, e),
                    ))
                }
            }
        }
        None => {
            if !matches.is_present(options::COMMAND) {
                println!("{}", niceness);
                return Ok(());
            }
            10_i32
        }
    };

    niceness += adjustment;
    if unsafe { libc::setpriority(PRIO_PROCESS, 0, niceness) } == -1 {
        show_warning!("setpriority: {}", Error::last_os_error());
    }

    let cstrs: Vec<CString> = matches
        .values_of(options::COMMAND)
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

pub fn uu_app<'a>() -> Command<'a> {
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
                .takes_value(true)
                .allow_hyphen_values(true),
        )
        .arg(Arg::new(options::COMMAND).multiple_occurrences(true))
}
