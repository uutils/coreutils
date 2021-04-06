//  * This file is part of the uutils coreutils package.
//  *
//  * (c) KokaKiwi <kokakiwi@kokakiwi.net>
//  * (c) Jian Zeng <anonymousknight86@gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

/* last synced with: whoami (GNU coreutils) 8.22 */
// Allow dead code here in order to keep all fields, constants here, for consistency.
#![allow(dead_code)]

#[macro_use]
extern crate uucore;

use uucore::utmpx::*;

use clap::{App, Arg};

static ABOUT: &str = "Display who is currently logged in, according to FILE.";
static VERSION: &str = env!("CARGO_PKG_VERSION");

static ARG_FILES: &str = "files";

fn get_usage() -> String {
    format!("{0} [FILE]", executable!())
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let usage = get_usage();

    let matches = App::new(executable!())
        .version(VERSION)
        .about(ABOUT)
        .usage(&usage[..])
        .arg(Arg::with_name(ARG_FILES).takes_value(true).max_values(1))
        .get_matches_from(args);

    let files: Vec<String> = matches
        .values_of(ARG_FILES)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_default();

    let filename = if !files.is_empty() {
        files[0].as_ref()
    } else {
        DEFAULT_FILE
    };

    let mut users = Utmpx::iter_all_records()
        .read_from(filename)
        .filter(Utmpx::is_user_process)
        .map(|ut| ut.user())
        .collect::<Vec<_>>();

    if !users.is_empty() {
        users.sort();
        println!("{}", users.join(" "));
    }

    0
}
