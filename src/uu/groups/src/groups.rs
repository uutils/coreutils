// This file is part of the uutils coreutils package.
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


use quick_error::quick_error;
use uucore::{
    display::Quotable,
    entries::{get_groups_gnu, gid2grp, Locate, Passwd},
    error::{UError, UResult},
    format_usage, help_about, help_usage, show,
};

use clap::{crate_version, Arg, ArgAction, Command};

mod options {
    pub const USERS: &str = "USERNAME";
}
const ABOUT: &str = help_about!("groups.md");
const USAGE: &str = help_usage!("groups.md");

quick_error! {
    #[derive(Debug)]
    pub enum GroupsError {
        GetGroupsFailed {
            display("failed to fetch groups")
        }
        GroupNotFound(gid: u32) {
            display("cannot find name for group ID {gid}")
        }
        UserNotFound(user: String) {
            display("{}: no such user", user.quote())
        }
    }
}

impl UError for GroupsError {}

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
    let matches = uu_app().try_get_matches_from(args)?;

    let users: Vec<String> = matches
        .get_many::<String>(options::USERS)
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

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::USERS)
                .action(ArgAction::Append)
                .value_name(options::USERS)
                .value_hint(clap::ValueHint::Username),
        )
}
