#![crate_name = "logname"]
#![feature(collections, core, io, libc, rustc_private, std_misc)]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Benoit Benedetti <benoit.benedetti@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

/* last synced with: logname (GNU coreutils) 8.22 */

#![allow(non_camel_case_types)]

extern crate getopts;
extern crate libc;

use std::ffi::c_str_to_bytes;
use std::old_io::print;
use libc::c_char;

#[path = "../common/util.rs"] #[macro_use] mod util;

extern {
    // POSIX requires using getlogin (or equivalent code)
    pub fn getlogin() -> *const libc::c_char;
}

fn get_userlogin() -> Option<String> {
    unsafe {
            let login: *const libc::c_char = getlogin();
            if login.is_null() {
                    None
            } else {
                    Some(String::from_utf8_lossy(c_str_to_bytes(&login)).to_string())
            }
    }
}

static NAME: &'static str = "logname";
static VERSION: &'static str = "1.0.0";

fn version() {
    println!("{} {}", NAME, VERSION);
}

pub fn uumain(args: Vec<String>) -> isize {
    let program = args[0].clone();

    //
    // Argument parsing
    //
    let opts = [
        getopts::optflag("h", "help", "display this help and exit"),
        getopts::optflag("V", "version", "output version information and exit"),
    ];

    let matches = match getopts::getopts(args.tail(), &opts) {
        Ok(m) => m,
        Err(f) => crash!(1, "Invalid options\n{}", f)
    };

    if matches.opt_present("help") {
        version();
        println!("");
        println!("Usage:");
        println!("  {}", program);
        println!("");
        print(getopts::usage("print user's login name", &opts).as_slice());
        return 0;
    }
    if matches.opt_present("version") {
        version();
        return 0;
    }

    exec();

    0
}

fn exec() {
        match get_userlogin() {
                Some(userlogin) => println!("{}", userlogin),
                None => println!("{}: no login name", NAME)
        }
}
