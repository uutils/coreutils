#[crate_id(name="tty", version="1.0.0", author="Alan Andrade")];


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

extern mod extra;

use std::{libc,str,os};
use std::io::println;
use std::io::stdio::stderr;
use extra::getopts::{optflag,getopts};

extern {
    fn ttyname(filedesc: libc::c_int) -> *libc::c_char;
    fn isatty(filedesc: libc::c_int) -> libc::c_int;
}

fn main () {
    let args = os::args();

    let options = [
        optflag("s")
    ];

    let silent = match getopts(args.tail(), options) {
        Ok(m) => {
            m.opt_present("s")
        },
        Err(f) => {
            println(f.to_err_msg());
            usage();
            return
        }
    };

    let tty = unsafe { str::raw::from_c_str(ttyname(libc::STDIN_FILENO)) };

    if !silent {
        if !tty.is_whitespace() {
            println(tty);
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

    os::set_exit_status(exit_code as int);
}

fn usage () {
    writeln!(&mut stderr() as &mut Writer, "usage: tty [-s]");
    os::set_exit_status(2);
}
