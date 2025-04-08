// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) COMFOLLOW Chowner RFILE RFILE's derefer dgid nonblank nonprint nonprinting

use uucore::display::Quotable;
pub use uucore::entries;
use uucore::error::{FromIo, UResult, USimpleError};
use uucore::perms::{GidUidOwnerFilter, IfFrom, chown_base, options};
use uucore::{format_usage, help_about, help_usage};

use clap::{Arg, ArgAction, ArgMatches, Command};

use std::fs;
use std::os::unix::fs::MetadataExt;

const ABOUT: &str = help_about!("chgrp.md");
const USAGE: &str = help_usage!("chgrp.md");

fn parse_gid_from_str(group: &str) -> Result<u32, String> {
    if let Some(gid_str) = group.strip_prefix(':') {
        // Handle :gid format
        gid_str
            .parse::<u32>()
            .map_err(|_| format!("invalid group id: '{gid_str}'"))
    } else {
        // Try as group name first
        match entries::grp2gid(group) {
            Ok(g) => Ok(g),
            // If group name lookup fails, try parsing as raw number
            Err(_) => group
                .parse::<u32>()
                .map_err(|_| format!("invalid group: '{group}'")),
        }
    }
}

fn get_dest_gid(matches: &ArgMatches) -> UResult<(Option<u32>, String)> {
    let mut raw_group = String::new();
    let dest_gid = if let Some(file) = matches.get_one::<String>(options::REFERENCE) {
        fs::metadata(file)
            .map(|meta| {
                let gid = meta.gid();
                raw_group = entries::gid2grp(gid).unwrap_or_else(|_| gid.to_string());
                Some(gid)
            })
            .map_err_context(|| format!("failed to get attributes of {}", file.quote()))?
    } else {
        let group = matches
            .get_one::<String>(options::ARG_GROUP)
            .map(|s| s.as_str())
            .unwrap_or_default();
        raw_group = group.to_string();
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
    let (dest_gid, raw_group) = get_dest_gid(matches)?;

    // Handle --from option
    let filter = if let Some(from_group) = matches.get_one::<String>(options::FROM) {
        match parse_gid_from_str(from_group) {
            Ok(g) => IfFrom::Group(g),
            Err(_) => {
                return Err(USimpleError::new(
                    1,
                    format!("invalid user: '{from_group}'"),
                ));
            }
        }
    } else {
        IfFrom::All
    };

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
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .disable_help_flag(true)
        .arg(
            Arg::new(options::HELP)
                .long(options::HELP)
                .help("Print help information.")
                .action(ArgAction::Help),
        )
        .arg(
            Arg::new(options::verbosity::CHANGES)
                .short('c')
                .long(options::verbosity::CHANGES)
                .help("like verbose but report only when a change is made")
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
                .help("suppress most error messages")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::verbosity::VERBOSE)
                .short('v')
                .long(options::verbosity::VERBOSE)
                .help("output a diagnostic for every file processed")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::preserve_root::PRESERVE)
                .long(options::preserve_root::PRESERVE)
                .help("fail to operate recursively on '/'")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::preserve_root::NO_PRESERVE)
                .long(options::preserve_root::NO_PRESERVE)
                .help("do not treat '/' specially (the default)")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::REFERENCE)
                .long(options::REFERENCE)
                .value_name("RFILE")
                .value_hint(clap::ValueHint::FilePath)
                .help("use RFILE's group rather than specifying GROUP values"),
        )
        .arg(
            Arg::new(options::FROM)
                .long(options::FROM)
                .value_name("GROUP")
                .help("change the group only if its current group matches GROUP"),
        )
        .arg(
            Arg::new(options::RECURSIVE)
                .short('R')
                .long(options::RECURSIVE)
                .help("operate on files and directories recursively")
                .action(ArgAction::SetTrue),
        )
        // Add common arguments with chgrp, chown & chmod
        .args(uucore::perms::common_args())
}
