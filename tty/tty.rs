#![crate_id(name="tty", version="1.0.0", author="Alan Andrade")]


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

#![allow(dead_code)]

#![feature(macro_rules)]

extern crate getopts;
extern crate libc;

use std::{str,os};
use std::io::println;
use std::io::stdio::stderr;
use getopts::{optflag,getopts};

#[path = "../common/util.rs"]
mod util;

extern {
    fn ttyname(filedesc: libc::c_int) -> *libc::c_char;
    fn isatty(filedesc: libc::c_int) -> libc::c_int;
}

static NAME: &'static str = "tty";

#[allow(dead_code)]
fn main () { os::set_exit_status(uumain(os::args())); }

pub fn uumain(args: Vec<String>) -> int {
    let options = [
        optflag("s", "silent", "print nothing, only return an exit status")
    ];

    let silent = match getopts(args.tail(), options) {
        Ok(m) => {
            m.opt_present("s")
        },
        Err(f) => {
            println(f.to_err_msg().as_slice());
            usage();
            return 2;
        }
    };

    let tty = unsafe { str::raw::from_c_str(ttyname(libc::STDIN_FILENO)) };

    if !silent {
        if !tty.as_slice().is_whitespace() {
            println(tty.as_slice());
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

    return exit_code as int;
}

fn usage () {
    safe_writeln!(&mut stderr() as &mut Writer, "usage: tty [-s]");
}
