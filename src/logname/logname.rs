#![crate_name = "logname"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Benoit Benedetti <benoit.benedetti@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

/* last synced with: logname (GNU coreutils) 8.22 */

extern crate getopts;
extern crate libc;

#[macro_use]
extern crate uucore;

use std::ffi::CStr;
use std::io::Write;

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
            Some(String::from_utf8_lossy(CStr::from_ptr(login).to_bytes()).to_string())
        }
    }
}

static NAME: &'static str = "logname";
static VERSION: &'static str = "1.0.0";

pub fn uumain(args: Vec<String>) -> i32 {
    //
    // Argument parsing
    //
    let mut opts = getopts::Options::new();

    opts.optflag("h", "help", "display this help and exit");
    opts.optflag("V", "version", "output version information and exit");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => crash!(1, "Invalid options\n{}", f)
    };

    if matches.opt_present("help") {
        let msg = format!("{0} {1}

Usage:
  {0}

Print user's login name.", NAME, VERSION);

        print!("{}", opts.usage(&msg));
        return 0;
    }
    if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
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

#[allow(dead_code)]
fn main() {
    std::process::exit(uumain(std::env::args().collect()));
}
