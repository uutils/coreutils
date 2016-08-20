#![crate_name = "uu_link"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Michael Gehring <mg@ebfe.org>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

#[macro_use]
extern crate uucore;

use std::fs::hard_link;
use std::io::Write;
use std::path::Path;
use std::io::Error;

static SYNTAX: &'static str = "[OPTIONS] FILE1 FILE2"; 
static SUMMARY: &'static str = "Create a link named FILE2 to FILE1"; 
static LONG_HELP: &'static str = ""; 

pub fn normalize_error_message(e: Error) -> String {
    match e.raw_os_error() {
        Some(2) => { String::from("No such file or directory (os error 2)") }
        _ => { format!("{}", e) }
    }
}

pub fn uumain(args: Vec<String>) -> i32 {
    let matches = new_coreopts!(SYNTAX, SUMMARY, LONG_HELP)
        .parse(args);
    if matches.free.len() != 2 {
        crash!(1, "{}", msg_wrong_number_of_arguments!(2));
    }

    let old = Path::new(&matches.free[0]);
    let new = Path::new(&matches.free[1]);

    match hard_link(old, new) {
        Ok(_) => 0,
        Err(err) => {
            show_error!("{}",normalize_error_message(err));
            1
        }
    }
}
