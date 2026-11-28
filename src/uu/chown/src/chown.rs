// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) Passwd RFILE RFILE's derefer dgid duid groupname

use uucore::display::Quotable;
pub use uucore::entries::{self, Group, Locate, Passwd};
use uucore::error::{FromIo, UResult, USimpleError};
use uucore::perms::{GidUidOwnerFilter, IfFrom, chown_base, options};
use uucore::{format_usage, show_warning, translate};

use clap::{Arg, ArgAction, ArgMatches, Command};

use std::ffi::OsString;
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
    if let Some(file) = matches.get_one::<OsString>(options::REFERENCE) {
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
        let owner = matches.get_one::<String>(options::ARG_OWNER).unwrap();
        let spec = parse_spec_with_display(owner, ':')?;
        raw_owner = spec.display;
        dest_uid = spec.uid;
        dest_gid = spec.gid;
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
    Command::new("chown")
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template("chown"))
        .about(translate!("chown-about"))
        .override_usage(format_usage(&translate!("chown-usage")))
        .infer_long_args(true)
        .args_override_self(true)
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
                .value_hint(clap::ValueHint::FilePath)
                .value_parser(clap::value_parser!(OsString)),
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

fn display_id(input: &str, id: u32, numeric: Option<u32>) -> String {
    if input.starts_with('+') && numeric.is_some() {
        id.to_string()
    } else {
        input.to_owned()
    }
}

/// Parses the user string to extract the UID and its display form.
fn parse_uid(user: &str, spec: &str) -> UResult<(Option<u32>, String)> {
    if user.is_empty() {
        return Ok((None, String::new()));
    }

    let numeric = user.parse::<u32>().ok();
    let uid = Passwd::locate_name(user)
        .ok()
        .map(|u| u.uid)
        .or_else(|| numeric.map(|id| Passwd::locate(id).map_or(id, |u| u.uid)))
        .ok_or_else(|| {
            USimpleError::new(
                1,
                translate!("chown-error-invalid-user", "user" => spec.quote()),
            )
        })?;
    Ok((Some(uid), display_id(user, uid, numeric)))
}

/// Parses the group string to extract the GID and its display form.
fn parse_gid(group: &str, spec: &str) -> UResult<(Option<u32>, String)> {
    if group.is_empty() {
        return Ok((None, String::new()));
    }

    let numeric = group.parse::<u32>().ok();
    let gid = Group::locate_name(group)
        .ok()
        .map(|g| g.gid)
        .or_else(|| numeric.map(|id| Group::locate(id).map_or(id, |g| g.gid)))
        .ok_or_else(|| {
            USimpleError::new(
                1,
                translate!("chown-error-invalid-group", "group" => spec.quote()),
            )
        })?;
    Ok((Some(gid), display_id(group, gid, numeric)))
}

#[derive(Debug)]
struct ParsedSpec {
    uid: Option<u32>,
    gid: Option<u32>,
    display: String,
}

/// Parse the owner/group specifier string into a user ID and a group ID.
///
/// The `spec` can be of the form:
///
/// * `"owner:group"`,
/// * `"owner"`,
/// * `":group"`,
/// * `"owner:"`,
///
/// and the owner or group can be specified either as an ID or a
/// name. The `sep` argument specifies which character to use as a
/// separator between the owner and group; calling code should set
/// this to `':'`. If a separator is given but the group is omitted
/// (as in `"owner:"`), the group is set to the owner's login group.
fn parse_spec(spec: &str, sep: char) -> UResult<(Option<u32>, Option<u32>)> {
    let spec = parse_spec_with_display(spec, sep)?;
    Ok((spec.uid, spec.gid))
}

fn parse_spec_with_display(spec: &str, sep: char) -> UResult<ParsedSpec> {
    assert!(['.', ':'].contains(&sep));
    let mut args = spec.splitn(2, sep);
    let user = args.next().unwrap_or("");
    let group = args.next().unwrap_or("");

    // dot separator: try as username first, fall back to owner.group (like GNU)
    if sep == ':' && !spec.contains(':') && spec.contains('.') {
        if let Ok((uid, display)) = parse_uid(user, spec) {
            return Ok(ParsedSpec {
                uid,
                gid: None,
                display,
            });
        }
        show_warning!(
            "{}",
            translate!("chown-warning-dot-separator", "spec" => spec.quote())
        );
        return parse_spec_with_display(spec, '.');
    }

    if !user.is_empty() && group.is_empty() && spec != user {
        // A separator was given but the group was not ("owner:"), so the
        // group is set to the owner's login group.
        return match Passwd::locate_name(user) {
            // Lookup must occur through name, not ID, to be valid here.
            Ok(passwd) if passwd.name == user => Ok(ParsedSpec {
                uid: Some(passwd.uid),
                gid: Some(passwd.gid),
                display: format!("{}{sep}", display_id(user, passwd.uid, user.parse().ok())),
            }),
            _ => Err(USimpleError::new(
                1,
                translate!("chown-error-invalid-spec", "spec" => spec.quote()),
            )),
        };
    }

    let (uid, user_display) = parse_uid(user, spec)?;
    let (gid, group_display) = parse_gid(group, spec)?;
    let display = if uid.is_none() && gid.is_some() {
        group_display
    } else if spec != user {
        format!("{user_display}{sep}{group_display}")
    } else {
        user_display
    };
    Ok(ParsedSpec { uid, gid, display })
}

#[cfg(test)]
mod test {
    use super::*;
    use uucore::locale;

    #[test]
    fn test_parse_spec() {
        temp_env::with_var("LANG", Some("C"), || {
            let _ = locale::setup_localization("chown");
        });
        assert!(matches!(parse_spec(":", ':'), Ok((None, None))));
        assert!(matches!(parse_spec(".", ':'), Ok((None, None))));
        assert!(matches!(parse_spec(".", '.'), Ok((None, None))));
        assert!(format!("{}", parse_spec("::", ':').err().unwrap()).starts_with("invalid group: "));
        assert!(format!("{}", parse_spec("..", ':').err().unwrap()).starts_with("invalid group: "));
    }

    /// Test that "USER:" sets the group to the user's login group.
    #[test]
    fn test_parse_spec_login_group() {
        if let Ok(passwd) = Passwd::locate("root") {
            let (uid, gid) = parse_spec("root:", ':').unwrap();
            assert_eq!(uid, Some(passwd.uid));
            assert_eq!(gid, Some(passwd.gid));
        }
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

    #[test]
    fn test_display_id_keeps_plus_prefixed_names() {
        assert_eq!(display_id("+alice", 42, None), "+alice");
        assert_eq!(display_id("+4294967296", 42, None), "+4294967296");
        assert_eq!(display_id("+00042", 42, Some(42)), "42");
    }
}
