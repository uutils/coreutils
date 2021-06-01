//  * This file is part of the uutils coreutils package.
//  *
//  * (c) KokaKiwi <kokakiwi@kokakiwi.net>
//  * (c) Jian Zeng <anonymousknight86@gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (paths) wtmp

#[macro_use]
extern crate uucore;

use clap::{App, Arg};
use uucore::utmpx::{self, Utmpx};

static VERSION: &str = env!("CARGO_PKG_VERSION");
static ABOUT: &str = "Print the user names of users currently logged in to the current host";

static ARG_FILES: &str = "files";

fn get_usage() -> String {
    format!("{0} [FILE]", executable!())
}

fn get_long_usage() -> String {
    format!(
        "Output who is currently logged in according to FILE.
If FILE is not specified, use {}.  /var/log/wtmp as FILE is common.",
        utmpx::DEFAULT_FILE
    )
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let usage = get_usage();
    let after_help = get_long_usage();

    let matches = App::new(executable!())
        .version(VERSION)
        .about(ABOUT)
        .usage(&usage[..])
        .after_help(&after_help[..])
        .arg(Arg::with_name(ARG_FILES).takes_value(true).max_values(1))
        .get_matches_from(args);

    let files: Vec<String> = matches
        .values_of(ARG_FILES)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_default();

    let filename = if !files.is_empty() {
        files[0].as_ref()
    } else {
        utmpx::DEFAULT_FILE
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
