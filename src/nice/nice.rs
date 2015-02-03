#![crate_name = "nice"]
#![feature(collections, core, libc, os, rustc_private, std_misc)]

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

use std::ffi::CString;
use std::old_io::IoError;
use std::os;
use libc::{c_char, c_int, execvp};

const NAME: &'static str = "nice";
const VERSION: &'static str = "1.0.0";

// XXX: PRIO_PROCESS is 0 on at least FreeBSD and Linux.  Don't know about Mac OS X.
const PRIO_PROCESS: c_int = 0;

#[path = "../common/util.rs"]
#[macro_use]
mod util;

extern {
    fn getpriority(which: c_int, who: c_int) -> c_int;
    fn setpriority(which: c_int, who: c_int, prio: c_int) -> c_int;
}

pub fn uumain(args: Vec<String>) -> isize {
    let opts = [
        getopts::optopt("n", "adjustment", "add N to the niceness (default is 10)", "N"),
        getopts::optflag("h", "help", "display this help and exit"),
        getopts::optflag("V", "version", "output version information and exit"),
    ];

    let matches = match getopts::getopts(args.tail(), &opts) {
        Ok(m) => m,
        Err(err) => {
            show_error!("{}", err);
            return 125;
        }
    };

    if matches.opt_present("version") || matches.opt_present("help") {
        println!("{} v{}", NAME, VERSION);
        if matches.opt_present("help") {
            let usage = getopts::usage("Run COMMAND with an adjusted niceness, \
                                        which affects process scheduling.\n\
                                        With no COMMAND, print the current \
                                        niceness.  Niceness values range from \
                                        at\nleast -20 (most favorable to the \
                                        process) to 19 (least favorable to the\
                                        \nprocess).", &opts);
            println!("");
            println!("Usage:");
            println!("  {} [OPTIONS] [COMMAND [ARGS]]", NAME);
            println!("");
            print!("{}", usage);
        }
        0
    } else {
        let mut niceness = unsafe { getpriority(PRIO_PROCESS, 0) };
        if os::errno() != 0 {
            show_error!("{}", IoError::last_error());
            return 125;
        }

        let adjustment = match matches.opt_str("adjustment") {
            Some(nstr) => {
                if matches.free.len() == 0 {
                    show_error!("A command must be given with an adjustment.
                                 Try \"{} --help\" for more information.", args[0]);
                    return 125;
                }
                match nstr.as_slice().parse() {
                    Ok(num) => num,
                    Err(e)=> {
                        show_error!("\"{}\" is not a valid number: {}", nstr, e);
                        return 125;
                    }
                }
            },
            None => {
                if matches.free.len() == 0 {
                    println!("{}", niceness);
                    return 0;
                }
                10 as c_int
            }
        };

        niceness += adjustment;
        unsafe { setpriority(PRIO_PROCESS, 0, niceness); }
        if os::errno() != 0 {
            show_warning!("{}", IoError::last_error());
        }

        let cstrs : Vec<CString> = matches.free.iter().map(|x| CString::from_slice(x.as_bytes())).collect();
        let mut args : Vec<*const c_char> = cstrs.iter().map(|s| s.as_ptr()).collect();
        args.push(0 as *const c_char);
        unsafe { execvp(args[0], args.as_mut_ptr()); }

        show_error!("{}", IoError::last_error());
        if os::errno() as c_int == libc::ENOENT { 127 } else { 126 }
    }
}
