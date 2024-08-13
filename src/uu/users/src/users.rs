// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (paths) wtmp

use std::ffi::OsString;
use std::path::Path;

use clap::builder::ValueParser;
use clap::{crate_version, Arg, Command};
use uucore::error::UResult;
use uucore::{format_usage, help_about, help_usage};

#[cfg(target_os = "openbsd")]
use utmp_classic::{parse_from_path, UtmpEntry};
#[cfg(not(target_os = "openbsd"))]
use uucore::utmpx::{self, Utmpx};

const ABOUT: &str = help_about!("users.md");
const USAGE: &str = help_usage!("users.md");

#[cfg(target_os = "openbsd")]
const OPENBSD_UTMP_FILE: &str = "/var/run/utmp";

static ARG_FILES: &str = "files";

fn get_long_usage() -> String {
    #[cfg(not(target_os = "openbsd"))]
    let default_path: &str = utmpx::DEFAULT_FILE;
    #[cfg(target_os = "openbsd")]
    let default_path: &str = OPENBSD_UTMP_FILE;
    format!(
        "Output who is currently logged in according to FILE.
If FILE is not specified, use {}.  /var/log/wtmp as FILE is common.",
        default_path
    )
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app()
        .after_help(get_long_usage())
        .try_get_matches_from(args)?;

    let files: Vec<&Path> = matches
        .get_many::<OsString>(ARG_FILES)
        .map(|v| v.map(AsRef::as_ref).collect())
        .unwrap_or_default();

    let mut users: Vec<String>;

    // OpenBSD uses the Unix version 1 UTMP, all other Unixes use the newer UTMPX
    #[cfg(target_os = "openbsd")]
    {
        let filename = if files.is_empty() {
            Path::new(OPENBSD_UTMP_FILE)
        } else {
            files[0]
        };
        let entries = parse_from_path(filename).unwrap_or(Vec::new());
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
        let filename = if files.is_empty() {
            utmpx::DEFAULT_FILE.as_ref()
        } else {
            files[0]
        };

        users = Utmpx::iter_all_records_from(filename)
            .filter(Utmpx::is_user_process)
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
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(ARG_FILES)
                .num_args(1)
                .value_hint(clap::ValueHint::FilePath)
                .value_parser(ValueParser::os_string()),
        )
}
