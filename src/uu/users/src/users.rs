// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (paths) wtmp

use std::ffi::OsString;
use std::path::Path;

use clap::builder::ValueParser;
use clap::{Arg, Command};
use uucore::error::UResult;
use uucore::format_usage;
use uucore::translate;

#[cfg(target_os = "openbsd")]
use utmp_classic::{UtmpEntry, parse_from_path};
use uucore::LocalizedCommand;
#[cfg(not(target_os = "openbsd"))]
use uucore::utmpx::{self, Utmpx};

#[cfg(target_os = "openbsd")]
const OPENBSD_UTMP_FILE: &str = "/var/run/utmp";

static ARG_FILE: &str = "file";

fn get_long_usage() -> String {
    #[cfg(not(target_os = "openbsd"))]
    let default_path: &str = utmpx::DEFAULT_FILE;
    #[cfg(target_os = "openbsd")]
    let default_path: &str = OPENBSD_UTMP_FILE;

    translate!("users-long-usage", "default_path" => default_path)
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app()
        .after_help(get_long_usage())
        .get_matches_from_localized(args);

    let maybe_file: Option<&Path> = matches.get_one::<OsString>(ARG_FILE).map(AsRef::as_ref);

    let mut users: Vec<String>;

    // OpenBSD uses the Unix version 1 UTMP, all other Unixes use the newer UTMPX
    #[cfg(target_os = "openbsd")]
    {
        let filename = maybe_file.unwrap_or(Path::new(OPENBSD_UTMP_FILE));
        let entries = parse_from_path(filename).unwrap_or_default();
        users = Vec::new();
        for entry in entries {
            if let UtmpEntry::UTMP {
                line: _,
                user,
                host: _,
                time: _,
            } = entry
            {
                if !user.is_empty() {
                    users.push(user);
                }
            }
        }
    };
    #[cfg(not(target_os = "openbsd"))]
    {
        let filename = maybe_file.unwrap_or(utmpx::DEFAULT_FILE.as_ref());

        users = Utmpx::iter_all_records_from(filename)
            .filter(|ut| ut.is_user_process())
            .map(|ut| ut.user())
            .collect::<Vec<_>>();
    };

    if !users.is_empty() {
        users.sort();
        println!("{}", users.join(" "));
    }

    Ok(())
}

pub fn uu_app() -> Command {
    #[cfg(not(target_env = "musl"))]
    let about = translate!("users-about");
    #[cfg(target_env = "musl")]
    let about = translate!("users-about") + &translate!("users-about-musl-warning");

    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .about(about)
        .override_usage(format_usage(&translate!("users-usage")))
        .infer_long_args(true)
        .arg(
            Arg::new(ARG_FILE)
                .num_args(1)
                .value_hint(clap::ValueHint::FilePath)
                .value_parser(ValueParser::os_string()),
        )
}
