// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) COMFOLLOW Chowner RFILE RFILE's derefer dgid nonblank nonprint nonprinting

#![cfg(unix)]

use uucore::display::Quotable;
use uucore::entries::{self, Group, Locate};
use uucore::error::{FromIo, UError, UResult, USimpleError};
use uucore::format_usage;
use uucore::perms::{GidUidOwnerFilter, IfFrom, chown_base, options};
use uucore::translate;

use clap::{Arg, ArgAction, ArgMatches, Command};

use std::ffi::{OsStr, OsString};
use std::fs;
use std::os::unix::fs::MetadataExt;

#[derive(Debug, thiserror::Error)]
enum ChgrpError {
    #[error("{}", translate!("chgrp-error-invalid-user", "from_group" => _0))]
    InvalidUser(String),
}

impl UError for ChgrpError {}

fn parse_gid_from_str(group: &OsStr) -> Result<u32, String> {
    if let Some(gid_str) = group.to_str().and_then(|s| s.strip_prefix(':')) {
        gid_str
            .parse::<u32>()
            .map_err(|_| translate!("chgrp-error-invalid-group-id", "gid_str" => gid_str))
    } else {
        Group::locate(group).map(|g| g.gid).or_else(|_| {
            group
                .to_str()
                .and_then(|s| s.parse::<u32>().ok())
                .ok_or_else(
                    || translate!("chgrp-error-invalid-group", "group" => group.to_string_lossy()),
                )
        })
    }
}

fn get_dest_gid(matches: &ArgMatches) -> UResult<(Option<u32>, String)> {
    let mut raw_group = String::new();
    let dest_gid = if let Some(file) = matches.get_one::<OsString>(options::REFERENCE) {
        let path = std::path::Path::new(file);
        fs::metadata(path)
            .map(|meta| {
                let gid = meta.gid();
                raw_group = entries::gid2grp(gid).unwrap_or_else(|_| gid.to_string());
                Some(gid)
            })
            .map_err_context(
                || translate!("chgrp-error-failed-to-get-attributes", "file" => path.quote()),
            )?
    } else {
        let group = matches
            .get_one::<OsString>(options::ARG_GROUP)
            .map(OsString::as_os_str)
            .unwrap_or_default();
        raw_group = group.to_string_lossy().into_owned();
        if group.is_empty() {
            None
        } else {
            match parse_gid_from_str(group) {
                Ok(g) => Some(g),
                Err(e) => return Err(USimpleError::new(1, e)),
            }
        }
    };
    Ok((dest_gid, raw_group))
}

fn parse_gid_and_uid(matches: &ArgMatches) -> UResult<GidUidOwnerFilter> {
    // Handle --from option
    let filter = if let Some(from_group) = matches.get_one::<OsString>(options::FROM) {
        let Ok(gid) = parse_gid_from_str(from_group) else {
            return Err(ChgrpError::InvalidUser(from_group.to_string_lossy().into_owned()).into());
        };
        IfFrom::Group(gid)
    } else {
        IfFrom::All
    };

    let (dest_gid, raw_group) = get_dest_gid(matches)?;

    Ok(GidUidOwnerFilter {
        dest_gid,
        dest_uid: None,
        raw_owner: raw_group,
        filter,
    })
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    chown_base(uu_app(), args, options::ARG_GROUP, parse_gid_and_uid, true)
}

pub fn uu_app() -> Command {
    let cmd = Command::new("chgrp")
        .version(uucore::crate_version!())
        .about(translate!("chgrp-about"))
        .override_usage(format_usage(&translate!("chgrp-usage")))
        .infer_long_args(true);
    uucore::clap_localization::configure_localized_command(cmd)
        .disable_help_flag(true)
        .arg(
            Arg::new(options::HELP)
                .long(options::HELP)
                .help(translate!("chgrp-help-print-help"))
                .action(ArgAction::Help),
        )
        .arg(
            Arg::new(options::verbosity::CHANGES)
                .short('c')
                .long(options::verbosity::CHANGES)
                .help(translate!("chgrp-help-changes"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::verbosity::SILENT)
                .short('f')
                .long(options::verbosity::SILENT)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::verbosity::QUIET)
                .long(options::verbosity::QUIET)
                .help(translate!("chgrp-help-quiet"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::verbosity::VERBOSE)
                .short('v')
                .long(options::verbosity::VERBOSE)
                .help(translate!("chgrp-help-verbose"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::preserve_root::PRESERVE)
                .long(options::preserve_root::PRESERVE)
                .help(translate!("chgrp-help-preserve-root"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::preserve_root::NO_PRESERVE)
                .long(options::preserve_root::NO_PRESERVE)
                .help(translate!("chgrp-help-no-preserve-root"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::REFERENCE)
                .long(options::REFERENCE)
                .value_name("RFILE")
                .value_hint(clap::ValueHint::FilePath)
                .value_parser(clap::value_parser!(OsString))
                .help(translate!("chgrp-help-reference")),
        )
        .arg(
            Arg::new(options::FROM)
                .long(options::FROM)
                .value_parser(clap::value_parser!(OsString))
                .value_name("GROUP")
                .help(translate!("chgrp-help-from")),
        )
        .arg(
            Arg::new(options::RECURSIVE)
                .short('R')
                .long(options::RECURSIVE)
                .help(translate!("chgrp-help-recursive"))
                .action(ArgAction::SetTrue),
        )
        // Add common arguments with chgrp, chown & chmod
        .args(uucore::perms::common_args())
}
