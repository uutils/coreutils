#![crate_name = "mkfifo"]
#![feature(macro_rules)]

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

use std::num::FromStrRadix;
use std::os;
use libc::funcs::posix88::stat_::mkfifo;

#[path = "../common/util.rs"]
mod util;

static NAME : &'static str = "mkfifo";
static VERSION : &'static str = "1.0.0";

pub fn uumain(args: Vec<String>) -> int {
    let opts = [
        getopts::optopt("m", "mode", "file permissions for the fifo", "(default 0666)"),
        getopts::optflag("h", "help", "display this help and exit"),
        getopts::optflag("V", "version", "output version information and exit"),
    ];

    let matches = match getopts::getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(err) => panic!("{}", err),
    };

    if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
        return 0;
    }

    if matches.opt_present("help") || matches.free.is_empty() {
        println!("{} {}", NAME, VERSION);
        println!("");
        println!("Usage:");
        println!("  {} [OPTIONS] NAME...", NAME);
        println!("");
        print!("{}", getopts::usage("Create a FIFO with the given name.", opts.as_slice()).as_slice());
        if matches.free.is_empty() {
            return 1;
        }
        return 0;
    }

    let mode = match matches.opt_str("m") {
        Some(m) => match FromStrRadix::from_str_radix(m.as_slice(), 8) {
            Some(m) => m,
            None => {
                show_error!("invalid mode");
                return 1;
            }
        },
        None => 0o666,
    };

    let mut exit_status = 0;
    for f in matches.free.iter() {
        f.with_c_str(|name| {
            let err = unsafe { mkfifo(name, mode) };
            if err == -1 {
                show_error!("creating '{}': {}", f, os::error_string(os::errno()));
                exit_status = 1;
            }
        });
    }

    exit_status
}
