#![crate_name = "users"]
#![feature(collections, core, io, libc, rustc_private, std_misc)]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) KokaKiwi <kokakiwi@kokakiwi.net>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

/* last synced with: whoami (GNU coreutils) 8.22 */

// Allow dead code here in order to keep all fields, constants here, for consistency.
#![allow(dead_code, non_camel_case_types)]

extern crate getopts;
extern crate libc;

use std::ffi::{CString, c_str_to_bytes};
use std::old_io::print;
use std::mem;
use std::ptr;
use utmpx::*;

#[path = "../common/util.rs"]
#[macro_use]
mod util;

#[path = "../common/utmpx.rs"]
mod utmpx;

extern {
    fn getutxent() -> *const c_utmp;
    fn getutxid(ut: *const c_utmp) -> *const c_utmp;
    fn getutxline(ut: *const c_utmp) -> *const c_utmp;

    fn pututxline(ut: *const c_utmp) -> *const c_utmp;

    fn setutxent();
    fn endutxent();

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    fn utmpxname(file: *const libc::c_char) -> libc::c_int;
}

#[cfg(target_os = "freebsd")]
unsafe extern fn utmpxname(_file: *const libc::c_char) -> libc::c_int {
    0
}

static NAME: &'static str = "users";

pub fn uumain(args: Vec<String>) -> isize {
    let program = args[0].as_slice();
    let opts = [
        getopts::optflag("h", "help", "display this help and exit"),
        getopts::optflag("V", "version", "output version information and exit"),
    ];

    let matches = match getopts::getopts(args.tail(), &opts) {
        Ok(m) => m,
        Err(f) => panic!("{}", f),
    };

    if matches.opt_present("help") {
        println!("users 1.0.0");
        println!("");
        println!("Usage:");
        println!("  {} [OPTION]... [FILE]", program);
        println!("");
        print(getopts::usage("Output who is currently logged in according to FILE.", &opts).as_slice());
        return 0;
    }

    if matches.opt_present("version") {
        println!("users 1.0.0");
        return 0;
    }

    let mut filename = DEFAULT_FILE;
    if matches.free.len() > 0 {
        filename = matches.free[0].as_slice();
    }

    exec(filename);

    0
}

fn exec(filename: &str) {
    unsafe {
        utmpxname(CString::from_slice(filename.as_bytes()).as_ptr());
    }

    let mut users = vec!();

    unsafe {
        setutxent();

        loop {
            let line = getutxent();

            if line == ptr::null() {
                break;
            }

            if (*line).ut_type == USER_PROCESS {
                let user = String::from_utf8_lossy(c_str_to_bytes(mem::transmute(&(*line).ut_user))).to_string();
                users.push(user);
            }
        }

        endutxent();
    }

    if users.len() > 0 {
        users.sort();
        println!("{}", users.connect(" "));
    }
}
