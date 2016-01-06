#![crate_name = "uu_tty"]

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

#[macro_use]
extern crate uucore;

use std::ffi::CStr;
use std::io::Write;
use uucore::fs::is_stdin_interactive;

extern {
    fn ttyname(filedesc: libc::c_int) -> *const libc::c_char;
}

static NAME: &'static str = "tty";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = getopts::Options::new();

    opts.optflag("s", "silent", "print nothing, only return an exit status");
    opts.optflag("h", "help", "display this help and exit");
    opts.optflag("V", "version", "output version information and exit");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => { crash!(2, "{}", f) }
    };

    if matches.opt_present("help") {
        println!("{} {}", NAME, VERSION);
        println!("");
        println!("Usage:");
        println!("  {} [OPTION]...", NAME);
        println!("");
        print!("{}", opts.usage("Print the file name of the terminal connected to standard input."));
    } else if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
    } else {
        let silent = matches.opt_present("s");

        let tty = unsafe {
            let ptr = ttyname(libc::STDIN_FILENO);
            if !ptr.is_null() {
                String::from_utf8_lossy(CStr::from_ptr(ptr).to_bytes()).to_string()
            } else {
                "".to_owned()
            }
        };

        if !silent {
            if !tty.chars().all(|c| c.is_whitespace()) {
                println!("{}", tty);
            } else {
                println!("not a tty");
            }
        }

        return if is_stdin_interactive() {
            libc::EXIT_SUCCESS
        } else {
            libc::EXIT_FAILURE
        };
    }

    0
}
