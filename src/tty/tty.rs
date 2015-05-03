#![crate_name = "tty"]
#![feature(rustc_private)]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Jordi Boggiano <j.boggiano@seld.be>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 *
 * Synced with http://lingrok.org/xref/coreutils/src/tty.c
 */

extern crate getopts;
extern crate libc;

use getopts::{getopts, optflag};
use std::ffi::CStr;
use std::io::Write;

#[path = "../common/util.rs"]
#[macro_use]
mod util;

extern {
    fn ttyname(filedesc: libc::c_int) -> *const libc::c_char;
    fn isatty(filedesc: libc::c_int) -> libc::c_int;
}

static NAME: &'static str = "tty";
static VERSION: &'static str = "1.0.0";

pub fn uumain(args: Vec<String>) -> i32 {
    let options = [
        optflag("s", "silent", "print nothing, only return an exit status"),
        optflag("h", "help", "display this help and exit"),
        optflag("V", "version", "output version information and exit")
    ];

    let matches = match getopts(&args[1..], &options) {
        Ok(m) => m,
        Err(f) => {
            crash!(2, "{}", f)
        }
    };

    if matches.opt_present("help") {
        let usage = getopts::usage("Print the file name of the terminal connected to standard input.", &options);

        println!("Usage: {} [OPTION]...\n{}", NAME, usage);
    } else if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
    } else {
        let silent = matches.opt_present("s");

        let tty = unsafe {
            let ptr = ttyname(libc::STDIN_FILENO);
            if !ptr.is_null() {
                String::from_utf8_lossy(CStr::from_ptr(ptr).to_bytes()).to_string()
            } else {
                "".to_string()
            }
        };

        if !silent {
            if !tty.chars().all(|c| c.is_whitespace()) {
                println!("{}", tty);
            } else {
                println!("not a tty");
            }
        }

        return unsafe {
            if isatty(libc::STDIN_FILENO) == 1 {
                libc::EXIT_SUCCESS
            } else {
                libc::EXIT_FAILURE
            }
        };
    }

    0
}
