// This file is part of the uutils coreutils package.
//
// (c) Alan Andrade <alan.andradec@gmail.com>
// (c) Jian Zeng <anonymousknight96 AT gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//
// ============================================================================
// Test suite summary for GNU coreutils 8.32.162-4eda
// ============================================================================
// PASS: tests/misc/groups-dash.sh
// PASS: tests/misc/groups-process-all.sh
// PASS: tests/misc/groups-version.sh

// spell-checker:ignore (ToDO) passwd

#[macro_use]
extern crate uucore;
use std::error::Error;
use std::fmt::Display;
use uucore::{
    display::Quotable,
    entries::{get_groups_gnu, gid2grp, Locate, Passwd},
    error::{UError, UResult},
    format_usage,
};

use clap::{crate_version, Arg, Command};

mod options {
    pub const USERS: &str = "USERNAME";
}
static ABOUT: &str = "Print group memberships for each USERNAME or, \
                      if no USERNAME is specified, for\nthe current process \
                      (which may differ if the groups data‐base has changed).";

const USAGE: &str = "{} [OPTION]... [USERNAME]...";

#[derive(Debug)]
enum GroupsError {
    GetGroupsFailed,
    GroupNotFound(u32),
    UserNotFound(String),
}

impl Error for GroupsError {}
impl UError for GroupsError {}

impl Display for GroupsError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            GroupsError::GetGroupsFailed => write!(f, "failed to fetch groups"),
            GroupsError::GroupNotFound(gid) => write!(f, "cannot find name for group ID {}", gid),
            GroupsError::UserNotFound(user) => write!(f, "{}: no such user", user.quote()),
        }
    }
}

fn infallible_gid2grp(gid: &u32) -> String {
    match gid2grp(*gid) {
        Ok(grp) => grp,
        Err(_) => {
            // The `show!()` macro sets the global exit code for the program.
            show!(GroupsError::GroupNotFound(*gid));
            gid.to_string()
        }
    }
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().get_matches_from(args);

    let users: Vec<String> = matches
        .values_of(options::USERS)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_default();

    if users.is_empty() {
        let gids = match get_groups_gnu(None) {
            Ok(v) => v,
            Err(_) => return Err(GroupsError::GetGroupsFailed.into()),
        };
        let groups: Vec<String> = gids.iter().map(infallible_gid2grp).collect();
        println!("{}", groups.join(" "));
        return Ok(());
    }

    for user in users {
        match Passwd::locate(user.as_str()) {
            Ok(p) => {
                let groups: Vec<String> = p.belongs_to().iter().map(infallible_gid2grp).collect();
                println!("{} : {}", user, groups.join(" "));
            }
            Err(_) => {
                // The `show!()` macro sets the global exit code for the program.
                show!(GroupsError::UserNotFound(user));
            }
        }
    }
    Ok(())
}

pub fn uu_app<'a>() -> Command<'a> {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::USERS)
                .multiple_occurrences(true)
                .takes_value(true)
                .value_name(options::USERS),
        )
}
