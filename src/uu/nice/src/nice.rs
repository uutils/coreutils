//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Alex Lyon <arcterus@mail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) getpriority execvp setpriority nstr PRIO cstrs ENOENT

#[macro_use]
extern crate uucore;

use libc::{c_char, c_int, execvp};
use std::ffi::CString;
use std::io::Error;
use std::ptr;

use clap::{App, AppSettings, Arg};
const VERSION: &str = env!("CARGO_PKG_VERSION");

// XXX: PRIO_PROCESS is 0 on at least FreeBSD and Linux.  Don't know about Mac OS X.
const PRIO_PROCESS: c_int = 0;

extern "C" {
    fn getpriority(which: c_int, who: c_int) -> c_int;
    fn setpriority(which: c_int, who: c_int, prio: c_int) -> c_int;
}

pub mod options {
    pub static ADJUSTMENT: &str = "adjustment";
    pub static COMMAND: &str = "COMMAND";
}

fn get_usage() -> String {
    format!(
        "
  {0} [OPTIONS] [COMMAND [ARGS]]

Run COMMAND with an adjusted niceness, which affects process scheduling.
With no COMMAND, print the current niceness.  Niceness values range from at
least -20 (most favorable to the process) to 19 (least favorable to the
process).",
        executable!()
    )
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let usage = get_usage();

    let matches = App::new(executable!())
        .setting(AppSettings::TrailingVarArg)
        .version(VERSION)
        .usage(&usage[..])
        .arg(
            Arg::with_name(options::ADJUSTMENT)
                .short("n")
                .long(options::ADJUSTMENT)
                .help("add N to the niceness (default is 10)")
                .takes_value(true)
                .allow_hyphen_values(true),
        )
        .arg(Arg::with_name(options::COMMAND).multiple(true))
        .get_matches_from(args);

    let mut niceness = unsafe {
        nix::errno::Errno::clear();
        getpriority(PRIO_PROCESS, 0)
    };
    if Error::last_os_error().raw_os_error().unwrap() != 0 {
        show_error!("getpriority: {}", Error::last_os_error());
        return 125;
    }

    let adjustment = match matches.value_of(options::ADJUSTMENT) {
        Some(nstr) => {
            if !matches.is_present(options::COMMAND) {
                show_error!(
                    "A command must be given with an adjustment.\nTry \"{} --help\" for more information.",
                    executable!()
                );
                return 125;
            }
            match nstr.parse() {
                Ok(num) => num,
                Err(e) => {
                    show_error!("\"{}\" is not a valid number: {}", nstr, e);
                    return 125;
                }
            }
        }
        None => {
            if !matches.is_present(options::COMMAND) {
                println!("{}", niceness);
                return 0;
            }
            10_i32
        }
    };

    niceness += adjustment;
    if unsafe { setpriority(PRIO_PROCESS, 0, niceness) } == -1 {
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
    if Error::last_os_error().raw_os_error().unwrap() as c_int == libc::ENOENT {
        127
    } else {
        126
    }
}
