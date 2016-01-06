#![crate_name = "uu_nice"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Arcterus <arcterus@mail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate getopts;
extern crate libc;

#[macro_use]
extern crate uucore;

use libc::{c_char, c_int, execvp};
use std::ffi::CString;
use std::io::{Error, Write};

const NAME: &'static str = "nice";
const VERSION: &'static str = env!("CARGO_PKG_VERSION");

// XXX: PRIO_PROCESS is 0 on at least FreeBSD and Linux.  Don't know about Mac OS X.
const PRIO_PROCESS: c_int = 0;

extern {
    fn getpriority(which: c_int, who: c_int) -> c_int;
    fn setpriority(which: c_int, who: c_int, prio: c_int) -> c_int;
}

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = getopts::Options::new();

    opts.optopt("n", "adjustment", "add N to the niceness (default is 10)", "N");
    opts.optflag("h", "help", "display this help and exit");
    opts.optflag("V", "version", "output version information and exit");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(err) => {
            show_error!("{}", err);
            return 125;
        }
    };

    if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
        return 0;
    }

    if matches.opt_present("help") {
        let msg = format!("{0} {1}

Usage:
  {0} [OPTIONS] [COMMAND [ARGS]]

Run COMMAND with an adjusted niceness, which affects process scheduling.
With no COMMAND, print the current niceness.  Niceness values range from at
least -20 (most favorable to the process) to 19 (least favorable to the
process).", NAME, VERSION);

        print!("{}", opts.usage(&msg));
        return 0;
    }

    let mut niceness = unsafe { getpriority(PRIO_PROCESS, 0) };
    if Error::last_os_error().raw_os_error().unwrap() != 0 {
        show_error!("{}", Error::last_os_error());
        return 125;
    }

    let adjustment = match matches.opt_str("adjustment") {
        Some(nstr) => {
            if matches.free.is_empty() {
                show_error!("A command must be given with an adjustment.
                                Try \"{} --help\" for more information.", args[0]);
                return 125;
            }
            match nstr.parse() {
                Ok(num) => num,
                Err(e)=> {
                    show_error!("\"{}\" is not a valid number: {}", nstr, e);
                    return 125;
                }
            }
        },
        None => {
            if matches.free.is_empty() {
                println!("{}", niceness);
                return 0;
            }
            10 as c_int
        }
    };

    niceness += adjustment;
    unsafe { setpriority(PRIO_PROCESS, 0, niceness); }
    if Error::last_os_error().raw_os_error().unwrap() != 0 {
        show_warning!("{}", Error::last_os_error());
    }

    let cstrs: Vec<CString> = matches.free.iter().map(|x| CString::new(x.as_bytes()).unwrap()).collect();
    let mut args: Vec<*const c_char> = cstrs.iter().map(|s| s.as_ptr()).collect();
    args.push(0 as *const c_char);
    unsafe { execvp(args[0], args.as_mut_ptr()); }

    show_error!("{}", Error::last_os_error());
    if Error::last_os_error().raw_os_error().unwrap() as c_int == libc::ENOENT { 127 } else { 126 }
}
