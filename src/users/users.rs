#![crate_name = "uu_users"]

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
#![allow(dead_code)]

extern crate getopts;
extern crate libc;

#[macro_use]
extern crate uucore;

use getopts::Options;
use std::ffi::{CStr, CString};
use std::mem;
use std::ptr;
use uucore::utmpx::*;

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
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = Options::new();

    opts.optflag("h", "help", "display this help and exit");
    opts.optflag("V", "version", "output version information and exit");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => panic!("{}", f),
    };

    if matches.opt_present("help") {
        println!("{} {}", NAME, VERSION);
        println!("");
        println!("Usage:");
        println!("  {} [OPTION]... [FILE]", NAME);
        println!("");
        println!("{}", opts.usage("Output who is currently logged in according to FILE."));
        return 0;
    }

    if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
        return 0;
    }

    let filename = if !matches.free.is_empty() {
        matches.free[0].as_ref()
    } else {
        DEFAULT_FILE
    };

    exec(filename);

    0
}

fn exec(filename: &str) {
    unsafe {
        utmpxname(CString::new(filename).unwrap().as_ptr());
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
                let user = String::from_utf8_lossy(CStr::from_ptr(mem::transmute(&(*line).ut_user)).to_bytes()).to_string();
                users.push(user);
            }
        }

        endutxent();
    }

    if !users.is_empty() {
        users.sort();
        println!("{}", users.join(" "));
    }
}
