#![crate_name = "uu_mkfifo"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Michael Gehring <mg@ebfe.org>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate getopts;
extern crate libc;

#[macro_use]
extern crate uucore;

use libc::mkfifo;
use std::ffi::CString;
use std::io::{Error, Write};

static NAME: &'static str = "mkfifo";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = getopts::Options::new();

    opts.optopt("m", "mode", "file permissions for the fifo", "(default 0666)");
    opts.optflag("h", "help", "display this help and exit");
    opts.optflag("V", "version", "output version information and exit");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(err) => panic!("{}", err),
    };

    if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
        return 0;
    }

    if matches.opt_present("help") || matches.free.is_empty() {
        let msg = format!("{0} {1}

Usage:
  {0} [OPTIONS] NAME...

Create a FIFO with the given name.", NAME, VERSION);

        print!("{}", opts.usage(&msg));
        if matches.free.is_empty() {
            return 1;
        }
        return 0;
    }

    let mode = match matches.opt_str("m") {
        Some(m) => match usize::from_str_radix(&m, 8) {
            Ok(m) => m,
            Err(e)=> {
                show_error!("invalid mode: {}", e);
                return 1;
            }
        },
        None => 0o666,
    };

    let mut exit_status = 0;
    for f in &matches.free {
        let err = unsafe { mkfifo(CString::new(f.as_bytes()).unwrap().as_ptr(), mode as libc::mode_t) };
        if err == -1 {
            show_error!("creating '{}': {}", f, Error::last_os_error().raw_os_error().unwrap());
            exit_status = 1;
        }
    }

    exit_status
}
