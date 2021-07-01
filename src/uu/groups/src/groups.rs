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
use uucore::entries::{get_groups_gnu, gid2grp, Locate, Passwd};

use clap::{crate_version, App, Arg};

mod options {
    pub const USERS: &str = "USERNAME";
}
static ABOUT: &str = "Print group memberships for each USERNAME or, \
                      if no USERNAME is specified, for\nthe current process \
                      (which may differ if the groups data‐base has changed).";

fn get_usage() -> String {
    format!("{0} [OPTION]... [USERNAME]...", executable!())
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let usage = get_usage();

    let matches = uu_app().usage(&usage[..]).get_matches_from(args);

    let users: Vec<String> = matches
        .values_of(options::USERS)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_default();

    let mut exit_code = 0;

    if users.is_empty() {
        println!(
            "{}",
            get_groups_gnu(None)
                .unwrap()
                .iter()
                .map(|&gid| gid2grp(gid).unwrap_or_else(|_| {
                    show_error!("cannot find name for group ID {}", gid);
                    exit_code = 1;
                    gid.to_string()
                }))
                .collect::<Vec<_>>()
                .join(" ")
        );
        return exit_code;
    }

    for user in users {
        if let Ok(p) = Passwd::locate(user.as_str()) {
            println!(
                "{} : {}",
                user,
                p.belongs_to()
                    .iter()
                    .map(|&gid| gid2grp(gid).unwrap_or_else(|_| {
                        show_error!("cannot find name for group ID {}", gid);
                        exit_code = 1;
                        gid.to_string()
                    }))
                    .collect::<Vec<_>>()
                    .join(" ")
            );
        } else {
            show_error!("'{}': no such user", user);
            exit_code = 1;
        }
    }
    exit_code
}

pub fn uu_app() -> App<'static, 'static> {
    App::new(util_name!())
        .version(crate_version!())
        .about(ABOUT)
        .arg(
            Arg::with_name(options::USERS)
                .multiple(true)
                .takes_value(true)
                .value_name(options::USERS),
        )
}
