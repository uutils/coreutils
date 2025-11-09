// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) COMFOLLOW Chowner RFILE RFILE's derefer dgid nonblank nonprint nonprinting

use uucore::display::Quotable;
use uucore::entries;
use uucore::error::{FromIo, UResult, USimpleError};
use uucore::format_usage;
use uucore::perms::{GidUidOwnerFilter, IfFrom, chown_base, options};
use uucore::translate;

use clap::{Arg, ArgAction, ArgMatches, Command};

use std::fs;
use std::os::unix::fs::MetadataExt;

fn parse_gid_from_str(group: &str) -> Result<u32, String> {
    if let Some(gid_str) = group.strip_prefix(':') {
        // Handle :gid format
        gid_str
            .parse::<u32>()
            .map_err(|_| translate!("chgrp-error-invalid-group-id", "gid_str" => gid_str))
    } else {
        // Try as group name first
        match entries::grp2gid(group) {
            Ok(g) => Ok(g),
            // If group name lookup fails, try parsing as raw number
            Err(_) => group
                .parse::<u32>()
                .map_err(|_| translate!("chgrp-error-invalid-group", "group" => group)),
        }
    }
}

fn get_dest_gid(matches: &ArgMatches) -> UResult<(Option<u32>, String)> {
    let mut raw_group = String::new();
    let dest_gid = if let Some(file) = matches.get_one::<std::ffi::OsString>(options::REFERENCE) {
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
                    translate!("chgrp-error-invalid-user", "from_group" => from_group),
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
    let cmd = Command::new(uucore::util_name())
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
                .value_parser(clap::value_parser!(std::ffi::OsString))
                .help(translate!("chgrp-help-reference")),
        )
        .arg(
            Arg::new(options::FROM)
                .long(options::FROM)
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
