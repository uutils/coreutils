// This file is part of the uutils coreutils package.
//
// (c) Jian Zeng <anonymousknight96@gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) COMFOLLOW Passwd RFILE RFILE's derefer dgid duid groupname

use uucore::display::Quotable;
pub use uucore::entries::{self, Group, Locate, Passwd};
use uucore::format_usage;
use uucore::perms::{chown_base, options, IfFrom};

use uucore::error::{FromIo, UResult, USimpleError};

use clap::{crate_version, Arg, ArgAction, ArgMatches, Command};

use std::fs;
use std::os::unix::fs::MetadataExt;

static ABOUT: &str = "change file owner and group";

const USAGE: &str = "\
    {} [OPTION]... [OWNER][:[GROUP]] FILE...
    {} [OPTION]... --reference=RFILE FILE...";

fn parse_gid_uid_and_filter(matches: &ArgMatches) -> UResult<(Option<u32>, Option<u32>, IfFrom)> {
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
    if let Some(file) = matches.get_one::<String>(options::REFERENCE) {
        let meta = fs::metadata(file)
            .map_err_context(|| format!("failed to get attributes of {}", file.quote()))?;
        dest_gid = Some(meta.gid());
        dest_uid = Some(meta.uid());
    } else {
        let (u, g) = parse_spec(matches.get_one::<String>(options::ARG_OWNER).unwrap(), ':')?;
        dest_uid = u;
        dest_gid = g;
    }
    Ok((dest_gid, dest_uid, filter))
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
        .version(crate_version!())
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
            Arg::new(options::dereference::DEREFERENCE)
                .long(options::dereference::DEREFERENCE)
                .help(
                    "affect the referent of each symbolic link (this is the default), \
                    rather than the symbolic link itself",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::dereference::NO_DEREFERENCE)
                .short('h')
                .long(options::dereference::NO_DEREFERENCE)
                .help(
                    "affect symbolic links instead of any referenced file \
                    (useful only on systems that can change the ownership of a symlink)",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::FROM)
                .long(options::FROM)
                .help(
                    "change the owner and/or group of each file only if its \
                    current owner and/or group match those specified here. \
                    Either may be omitted, in which case a match is not required \
                    for the omitted attribute",
                )
                .value_name("CURRENT_OWNER:CURRENT_GROUP"),
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
            Arg::new(options::verbosity::QUIET)
                .long(options::verbosity::QUIET)
                .help("suppress most error messages")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::RECURSIVE)
                .short('R')
                .long(options::RECURSIVE)
                .help("operate on files and directories recursively")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::REFERENCE)
                .long(options::REFERENCE)
                .help("use RFILE's owner and group rather than specifying OWNER:GROUP values")
                .value_name("RFILE")
                .value_hint(clap::ValueHint::FilePath)
                .num_args(1..),
        )
        .arg(
            Arg::new(options::verbosity::SILENT)
                .short('f')
                .long(options::verbosity::SILENT)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::traverse::TRAVERSE)
                .short(options::traverse::TRAVERSE.chars().next().unwrap())
                .help("if a command line argument is a symbolic link to a directory, traverse it")
                .overrides_with_all(&[options::traverse::EVERY, options::traverse::NO_TRAVERSE])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::traverse::EVERY)
                .short(options::traverse::EVERY.chars().next().unwrap())
                .help("traverse every symbolic link to a directory encountered")
                .overrides_with_all(&[options::traverse::TRAVERSE, options::traverse::NO_TRAVERSE])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::traverse::NO_TRAVERSE)
                .short(options::traverse::NO_TRAVERSE.chars().next().unwrap())
                .help("do not traverse any symbolic links (default)")
                .overrides_with_all(&[options::traverse::TRAVERSE, options::traverse::EVERY])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::verbosity::VERBOSE)
                .long(options::verbosity::VERBOSE)
                .short('v')
                .help("output a diagnostic for every file processed")
                .action(ArgAction::SetTrue),
        )
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

    let uid = if !user.is_empty() {
        Some(match Passwd::locate(user) {
            Ok(u) => u.uid, // We have been able to get the uid
            Err(_) =>
            // we have NOT been able to find the uid
            // but we could be in the case where we have user.group
            {
                if spec.contains('.') && !spec.contains(':') && sep == ':' {
                    // but the input contains a '.' but not a ':'
                    // we might have something like username.groupname
                    // So, try to parse it this way
                    return parse_spec(spec, '.');
                } else {
                    // It's possible that the `user` string contains a
                    // numeric user ID, in which case, we respect that.
                    match user.parse() {
                        Ok(uid) => uid,
                        Err(_) => {
                            return Err(USimpleError::new(
                                1,
                                format!("invalid user: {}", spec.quote()),
                            ))
                        }
                    }
                }
            }
        })
    } else {
        None
    };
    let gid = if !group.is_empty() {
        Some(match Group::locate(group) {
            Ok(g) => g.gid,
            Err(_) => match group.parse() {
                Ok(gid) => gid,
                Err(_) => {
                    return Err(USimpleError::new(
                        1,
                        format!("invalid group: {}", spec.quote()),
                    ));
                }
            },
        })
    } else {
        None
    };
    Ok((uid, gid))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_spec() {
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
