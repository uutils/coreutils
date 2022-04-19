//  * This file is part of the uutils coreutils package.
//  *
//  * (c) KokaKiwi <kokakiwi@kokakiwi.net>
//  * (c) Jian Zeng <anonymousknight86@gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (paths) wtmp

use std::path::Path;

use clap::{crate_version, Arg, Command};
use uucore::error::UResult;
use uucore::format_usage;
use uucore::utmpx::{self, Utmpx};

static ABOUT: &str = "Print the user names of users currently logged in to the current host";
const USAGE: &str = "{} [FILE]";

static ARG_FILES: &str = "files";

fn get_long_usage() -> String {
    format!(
        "Output who is currently logged in according to FILE.
If FILE is not specified, use {}.  /var/log/wtmp as FILE is common.",
        utmpx::DEFAULT_FILE
    )
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let after_help = get_long_usage();

    let matches = uu_app().after_help(&after_help[..]).get_matches_from(args);

    let files: Vec<&Path> = matches
        .values_of_os(ARG_FILES)
        .map(|v| v.map(AsRef::as_ref).collect())
        .unwrap_or_default();

    let filename = if !files.is_empty() {
        files[0]
    } else {
        utmpx::DEFAULT_FILE.as_ref()
    };

    let mut users = Utmpx::iter_all_records_from(filename)
        .filter(Utmpx::is_user_process)
        .map(|ut| ut.user())
        .collect::<Vec<_>>();

    if !users.is_empty() {
        users.sort();
        println!("{}", users.join(" "));
    }

    Ok(())
}

pub fn uu_app<'a>() -> Command<'a> {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(Arg::new(ARG_FILES).takes_value(true).max_values(1))
}
