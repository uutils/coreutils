//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Michael Gehring <mg@ebfe.org>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

#[macro_use]
extern crate uucore;

use clap::{App, Arg};
use std::fs::hard_link;
use std::io::Error;
use std::path::Path;

static VERSION: &str = env!("CARGO_PKG_VERSION");
static ABOUT: &str = "Call the link function to create a link named FILE2 to an existing FILE1.";

pub mod options {
    pub static FILES: &str = "FILES";
}

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
    let matches = App::new(executable!())
        .version(VERSION)
        .about(ABOUT)
        .usage(&usage[..])
        .arg(
            Arg::with_name(options::FILES)
                .hidden(true)
                .required(true)
                .min_values(2)
                .max_values(2)
                .takes_value(true),
        )
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
