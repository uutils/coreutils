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

use std::ffi::{CString, OsStr, OsString};
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;

fn parse_gid_uid_and_filter(matches: &ArgMatches) -> UResult<GidUidOwnerFilter> {
    let filter = if let Some(spec) = matches.get_one::<OsString>(options::FROM) {
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
        let spec = matches.get_one::<OsString>(options::ARG_OWNER).unwrap();
        let (u, g) = parse_spec(spec, ':')?;
        raw_owner = spec.to_string_lossy().into_owned();
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
    Command::new("chown")
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template("chown"))
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
                .value_name("CURRENT_OWNER:CURRENT_GROUP")
                .value_parser(clap::value_parser!(OsString)),
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

/// Parses the user string to extract the UID.
fn parse_uid(user: &OsStr, spec: &OsStr) -> UResult<Option<u32>> {
    if user.is_empty() {
        return Ok(None);
    }

    if let Ok(u) = Passwd::locate(user) {
        return Ok(Some(u.uid));
    }

    // Fallback: `user` string contains a numeric user ID
    user.to_str()
        .and_then(|s| s.parse().ok())
        .map(Some)
        .ok_or_else(|| {
            USimpleError::new(
                1,
                translate!("chown-error-invalid-user", "user" => spec.quote()),
            )
        })
}

/// Parses the group string to extract the GID.
fn parse_gid(group: &OsStr, spec: &OsStr) -> UResult<Option<u32>> {
    if group.is_empty() {
        return Ok(None);
    }
    match Group::locate(group) {
        Ok(g) => Ok(Some(g.gid)),
        Err(_) => match group.to_str().and_then(|s| s.parse().ok()) {
            Some(gid) => Ok(Some(gid)),
            None => Err(USimpleError::new(
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
/// * `"owner:"`,
///
/// and the owner or group can be specified either as an ID or a
/// name. The `sep` argument specifies which character to use as a
/// separator between the owner and group; calling code should set
/// this to `':'`. If a separator is given but the group is omitted
/// (as in `"owner:"`), the group is set to the owner's login group.
fn parse_spec(spec: &OsStr, sep: char) -> UResult<(Option<u32>, Option<u32>)> {
    assert!(['.', ':'].contains(&sep));
    let bytes = spec.as_bytes();
    let mut args = bytes.splitn(2, |&c| c == sep as u8);
    let user = OsStr::from_bytes(args.next().unwrap_or_default());
    let group = OsStr::from_bytes(args.next().unwrap_or_default());

    // dot separator: try as username first, fall back to owner.group (like GNU)
    if sep == ':' && !bytes.contains(&b':') && bytes.contains(&b'.') {
        if let Ok(uid) = parse_uid(user, spec) {
            let gid = parse_gid(group, spec)?;
            return Ok((uid, gid));
        }
        show_warning!(
            "{}",
            translate!("chown-warning-dot-separator", "spec" => spec.quote())
        );
        return parse_spec(spec, '.');
    }

    if !user.is_empty() && group.is_empty() && spec != user {
        // A separator was given but the group was not ("owner:"), so the
        // group is set to the owner's login group.
        // Resolve by name only: a numeric fallback must not supply a login group.
        let passwd = CString::new(user.as_bytes())
            .ok()
            .and_then(|name| Passwd::locate(name.as_c_str()).ok());
        return match passwd {
            Some(passwd) => Ok((Some(passwd.uid), Some(passwd.gid))),
            _ => Err(USimpleError::new(
                1,
                translate!("chown-error-invalid-spec", "spec" => spec.quote()),
            )),
        };
    }

    Ok((parse_uid(user, spec)?, parse_gid(group, spec)?))
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
        assert!(matches!(parse_spec(OsStr::new(":"), ':'), Ok((None, None))));
        assert!(matches!(parse_spec(OsStr::new("."), ':'), Ok((None, None))));
        assert!(matches!(parse_spec(OsStr::new("."), '.'), Ok((None, None))));
        assert!(
            format!("{}", parse_spec(OsStr::new("::"), ':').err().unwrap())
                .starts_with("invalid group: ")
        );
        assert!(
            format!("{}", parse_spec(OsStr::new(".."), ':').err().unwrap())
                .starts_with("invalid group: ")
        );
    }

    /// Test that "USER:" sets the group to the user's login group.
    #[test]
    fn test_parse_spec_login_group() {
        if let Ok(passwd) = Passwd::locate("root") {
            let (uid, gid) = parse_spec(OsStr::new("root:"), ':').unwrap();
            assert_eq!(uid, Some(passwd.uid));
            assert_eq!(gid, Some(passwd.gid));
        }
    }

    /// Test for parsing IDs that don't correspond to a named user or group.
    #[test]
    fn test_parse_spec_nameless_ids() {
        // This assumes that there is no named user with ID 12345.
        assert!(matches!(
            parse_spec(OsStr::new("12345"), ':'),
            Ok((Some(12345), None))
        ));
        // This assumes that there is no named group with ID 54321.
        assert!(matches!(
            parse_spec(OsStr::new(":54321"), ':'),
            Ok((None, Some(54321)))
        ));
        assert!(matches!(
            parse_spec(OsStr::new("12345:54321"), ':'),
            Ok((Some(12345), Some(54321)))
        ));
    }
}
