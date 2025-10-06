// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) passwd

use thiserror::Error;
use uucore::{
    display::Quotable,
    entries::{Locate, Passwd, get_groups_gnu, gid2grp},
    error::{UError, UResult},
    format_usage, show,
};

use clap::{Arg, ArgAction, Command};
use uucore::translate;

mod options {
    pub const USERS: &str = "USERNAME";
}

#[derive(Debug, Error)]
enum GroupsError {
    #[error("{message}", message = translate!("groups-error-fetch"))]
    GetGroupsFailed,

    #[error("{message} {gid}", message = translate!("groups-error-notfound"), gid = .0)]
    GroupNotFound(u32),

    #[error("{user}: {message}", user = .0.quote(), message = translate!("groups-error-user"))]
    UserNotFound(String),
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
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;

    let users: Vec<String> = matches
        .get_many::<String>(options::USERS)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_default();

    if users.is_empty() {
        let Ok(gids) = get_groups_gnu(None) else {
            return Err(GroupsError::GetGroupsFailed.into());
        };
        let groups: Vec<String> = gids.iter().map(infallible_gid2grp).collect();
        println!("{}", groups.join(" "));
        return Ok(());
    }

    for user in users {
        match Passwd::locate(user.as_str()) {
            Ok(p) => {
                let groups: Vec<String> = p.belongs_to().iter().map(infallible_gid2grp).collect();
                println!("{user} : {}", groups.join(" "));
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
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .about(translate!("groups-about"))
        .override_usage(format_usage(&translate!("groups-usage")))
        .infer_long_args(true)
        .arg(
            Arg::new(options::USERS)
                .action(ArgAction::Append)
                .value_name(options::USERS)
                .value_hint(clap::ValueHint::Username),
        )
}
