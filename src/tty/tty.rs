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

use std::ffi::CStr;
use getopts::{optflag,getopts};

#[path = "../common/util.rs"]
#[macro_use]
mod util;

extern {
    fn ttyname(filedesc: libc::c_int) -> *const libc::c_char;
    fn isatty(filedesc: libc::c_int) -> libc::c_int;
}

static NAME: &'static str = "tty";

pub fn uumain(args: Vec<String>) -> i32 {
    let options = [
        optflag("s", "silent", "print nothing, only return an exit status")
    ];

    let silent = match getopts(&args[1..], &options) {
        Ok(m) => {
            m.opt_present("s")
        },
        Err(f) => {
            println!("{}", f);
            usage();
            return 2;
        }
    };

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

    let exit_code = unsafe {
        if isatty(libc::STDIN_FILENO) == 1 {
            libc::EXIT_SUCCESS
        } else {
            libc::EXIT_FAILURE
        }
    };

    exit_code
}

fn usage() {
    println!("usage: {} [-s]", NAME);
}
