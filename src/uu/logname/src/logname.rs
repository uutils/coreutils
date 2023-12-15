// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) getlogin userlogin

use clap::{crate_version, Command};
use std::ffi::CStr;
use uucore::{error::UResult, format_usage, help_about, help_usage, show_error};

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

const ABOUT: &str = help_about!("logname.md");
const USAGE: &str = help_usage!("logname.md");

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let _ = uu_app().try_get_matches_from(args)?;

    match get_userlogin() {
        Some(userlogin) => println!("{userlogin}"),
        None => show_error!("no login name"),
    }

    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .override_usage(format_usage(USAGE))
        .about(ABOUT)
        .infer_long_args(true)
}
