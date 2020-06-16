//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Benoit Benedetti <benoit.benedetti@gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

/* last synced with: logname (GNU coreutils) 8.22 */

// spell-checker:ignore (ToDO) getlogin userlogin

extern crate libc;

#[macro_use]
extern crate uucore;

use std::ffi::CStr;

extern "C" {
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

static SYNTAX: &str = "";
static SUMMARY: &str = "Print user's login name";
static LONG_HELP: &str = "";

pub fn uumain(args: impl uucore::Args) -> i32 {
    app!(SYNTAX, SUMMARY, LONG_HELP).parse(args.collect_str());

    match get_userlogin() {
        Some(userlogin) => println!("{}", userlogin),
        None => show_error!("no login name"),
    }

    0
}
