//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Michael Gehring <mg@ebfe.org>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

#[macro_use]
extern crate uucore;

use std::fs::hard_link;
use std::io::Error;
use std::path::Path;

use crate::app::{get_app, options};

pub mod app;

fn get_usage() -> String {
    format!("{0} FILE1 FILE2", executable!())
}

pub fn normalize_error_message(e: Error) -> String {
    match e.raw_os_error() {
        Some(2) => String::from("No such file or directory (os error 2)"),
        _ => format!("{}", e),
    }
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let usage = get_usage();
    let matches = get_app(executable!())
        .usage(&usage[..])
        .get_matches_from(args);

    let files: Vec<_> = matches
        .values_of_os(options::FILES)
        .unwrap_or_default()
        .collect();
    let old = Path::new(files[0]);
    let new = Path::new(files[1]);

    match hard_link(old, new) {
        Ok(_) => 0,
        Err(err) => {
            show_error!("{}", normalize_error_message(err));
            1
        }
    }
}
