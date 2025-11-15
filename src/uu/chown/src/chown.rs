// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) COMFOLLOW Passwd RFILE RFILE's derefer dgid duid groupname

use uucore::display::Quotable;
pub use uucore::entries::{self, Group, Locate, Passwd};
use uucore::format_usage;
use uucore::perms::{GidUidOwnerFilter, IfFrom, chown_base, options};
use uucore::translate;

use uucore::error::{FromIo, UResult, USimpleError};

use clap::{Arg, ArgAction, ArgMatches, Command};

use std::fs;
use std::os::unix::fs::MetadataExt;

fn parse_gid_uid_and_filter(matches: &ArgMatches) -> UResult<GidUidOwnerFilter> {
    let filter = if let Some(spec) = matches.get_one::<String>(options::FROM) {
        match parse_spec(spec, ':')? {
            (Some(uid), None) => IfFrom::User(uid),
            (None, Some(gid)) => IfFrom::Group(gid),
            (Some(uid), Some(gid)) => IfFrom::UserGroup(uid, gid),
            (None, None) => IfFrom::All,
        }
    } else {
        IfFrom::All
    };

    let dest_uid: Option<u32>;
    let dest_gid: Option<u32>;
    let raw_owner: String;
    if let Some(file) = matches.get_one::<String>(options::REFERENCE) {
        let meta = fs::metadata(file).map_err_context(
            || translate!("chown-error-failed-to-get-attributes", "file" => file.quote()),
        )?;
        let gid = meta.gid();
        let uid = meta.uid();
        dest_gid = Some(gid);
        dest_uid = Some(uid);
        raw_owner = format!(
            "{}:{}",
            entries::uid2usr(uid).unwrap_or_else(|_| uid.to_string()),
            entries::gid2grp(gid).unwrap_or_else(|_| gid.to_string())
        );
    } else {
        raw_owner = matches
            .get_one::<String>(options::ARG_OWNER)
            .unwrap()
            .into();
        let (u, g) = parse_spec(&raw_owner, ':')?;
        dest_uid = u;
        dest_gid = g;
    }
    Ok(GidUidOwnerFilter {
        dest_gid,
        dest_uid,
        raw_owner,
        filter,
    })
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    chown_base(
        uu_app(),
        args,
        options::ARG_OWNER,
        parse_gid_uid_and_filter,
        false,
    )
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .about(translate!("chown-about"))
        .override_usage(format_usage(&translate!("chown-usage")))
        .infer_long_args(true)
        .disable_help_flag(true)
        .arg(
            Arg::new(options::HELP)
                .long(options::HELP)
                .help(translate!("chown-help-print-help"))
                .action(ArgAction::Help),
        )
        .arg(
            Arg::new(options::verbosity::CHANGES)
                .short('c')
                .long(options::verbosity::CHANGES)
                .help(translate!("chown-help-changes"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::FROM)
                .long(options::FROM)
                .help(translate!("chown-help-from"))
                .value_name("CURRENT_OWNER:CURRENT_GROUP"),
        )
        .arg(
            Arg::new(options::preserve_root::PRESERVE)
                .long(options::preserve_root::PRESERVE)
                .help(translate!("chown-help-preserve-root"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::preserve_root::NO_PRESERVE)
                .long(options::preserve_root::NO_PRESERVE)
                .help(translate!("chown-help-no-preserve-root"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::verbosity::QUIET)
                .long(options::verbosity::QUIET)
                .help(translate!("chown-help-quiet"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::RECURSIVE)
                .short('R')
                .long(options::RECURSIVE)
                .help(translate!("chown-help-recursive"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::REFERENCE)
                .long(options::REFERENCE)
                .help(translate!("chown-help-reference"))
                .value_name("RFILE")
                .value_hint(clap::ValueHint::FilePath),
        )
        .arg(
            Arg::new(options::verbosity::SILENT)
                .short('f')
                .long(options::verbosity::SILENT)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::verbosity::VERBOSE)
                .long(options::verbosity::VERBOSE)
                .short('v')
                .help(translate!("chown-help-verbose"))
                .action(ArgAction::SetTrue),
        )
        // Add common arguments with chgrp, chown & chmod
        .args(uucore::perms::common_args())
}

/// Parses the user string to extract the UID.
fn parse_uid(user: &str, spec: &str, sep: char) -> UResult<Option<u32>> {
    if user.is_empty() {
        return Ok(None);
    }
    match Passwd::locate(user) {
        Ok(u) => Ok(Some(u.uid)), // We have been able to get the uid
        Err(_) => {
            // we have NOT been able to find the uid
            // but we could be in the case where we have user.group
            if spec.contains('.') && !spec.contains(':') && sep == ':' {
                // but the input contains a '.' but not a ':'
                // we might have something like username.groupname
                // So, try to parse it this way
                parse_spec(spec, '.').map(|(uid, _)| uid)
            } else {
                // It's possible that the `user` string contains a
                // numeric user ID, in which case, we respect that.
                match user.parse() {
                    Ok(uid) => Ok(Some(uid)),
                    Err(_) => Err(USimpleError::new(
                        1,
                        translate!("chown-error-invalid-user", "user" => spec.quote()),
                    )),
                }
            }
        }
    }
}

/// Parses the group string to extract the GID.
fn parse_gid(group: &str, spec: &str) -> UResult<Option<u32>> {
    if group.is_empty() {
        return Ok(None);
    }
    match Group::locate(group) {
        Ok(g) => Ok(Some(g.gid)),
        Err(_) => match group.parse() {
            Ok(gid) => Ok(Some(gid)),
            Err(_) => Err(USimpleError::new(
                1,
                translate!("chown-error-invalid-group", "group" => spec.quote()),
            )),
        },
    }
}

/// Parse the owner/group specifier string into a user ID and a group ID.
///
/// The `spec` can be of the form:
///
/// * `"owner:group"`,
/// * `"owner"`,
/// * `":group"`,
///
/// and the owner or group can be specified either as an ID or a
/// name. The `sep` argument specifies which character to use as a
/// separator between the owner and group; calling code should set
/// this to `':'`.
fn parse_spec(spec: &str, sep: char) -> UResult<(Option<u32>, Option<u32>)> {
    assert!(['.', ':'].contains(&sep));
    let mut args = spec.splitn(2, sep);
    let user = args.next().unwrap_or("");
    let group = args.next().unwrap_or("");

    let uid = parse_uid(user, spec, sep)?;
    let gid = parse_gid(group, spec)?;

    if user.chars().next().is_some_and(char::is_numeric) && group.is_empty() && spec != user {
        // if the arg starts with an id numeric value, the group isn't set but the separator is provided,
        // we should fail with an error
        return Err(USimpleError::new(
            1,
            translate!("chown-error-invalid-spec", "spec" => spec.quote()),
        ));
    }

    Ok((uid, gid))
}

#[cfg(test)]
mod test {
    use super::*;
    use std::env;
    use uucore::locale;

    #[test]
    fn test_parse_spec() {
        unsafe {
            env::set_var("LANG", "C");
        }
        let _ = locale::setup_localization("chown");
        assert!(matches!(parse_spec(":", ':'), Ok((None, None))));
        assert!(matches!(parse_spec(".", ':'), Ok((None, None))));
        assert!(matches!(parse_spec(".", '.'), Ok((None, None))));
        assert!(format!("{}", parse_spec("::", ':').err().unwrap()).starts_with("invalid group: "));
        assert!(format!("{}", parse_spec("..", ':').err().unwrap()).starts_with("invalid group: "));
    }

    /// Test for parsing IDs that don't correspond to a named user or group.
    #[test]
    fn test_parse_spec_nameless_ids() {
        // This assumes that there is no named user with ID 12345.
        assert!(matches!(parse_spec("12345", ':'), Ok((Some(12345), None))));
        // This assumes that there is no named group with ID 54321.
        assert!(matches!(parse_spec(":54321", ':'), Ok((None, Some(54321)))));
        assert!(matches!(
            parse_spec("12345:54321", ':'),
            Ok((Some(12345), Some(54321)))
        ));
    }
}
