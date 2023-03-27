// This file is part of the uutils coreutils package.
//
// (c) Jian Zeng <anonymousknight96@gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) COMFOLLOW Chowner RFILE RFILE's derefer dgid nonblank nonprint nonprinting

use uucore::display::Quotable;
pub use uucore::entries;
use uucore::error::{FromIo, UResult, USimpleError};
use uucore::perms::{chown_base, options, IfFrom};
use uucore::{format_usage, help_about, help_usage};

use clap::{crate_version, Arg, ArgAction, ArgMatches, Command};

use std::fs;
use std::os::unix::fs::MetadataExt;

static ABOUT: &str = help_about!("chgrp.md");
const USAGE: &str = help_usage!("chgrp.md");

fn parse_gid_and_uid(matches: &ArgMatches) -> UResult<(Option<u32>, Option<u32>, IfFrom)> {
    let dest_gid = if let Some(file) = matches.get_one::<String>(options::REFERENCE) {
        fs::metadata(file)
            .map(|meta| Some(meta.gid()))
            .map_err_context(|| format!("failed to get attributes of {}", file.quote()))?
    } else {
        let group = matches
            .get_one::<String>(options::ARG_GROUP)
            .map(|s| s.as_str())
            .unwrap_or_default();
        if group.is_empty() {
            None
        } else {
            match entries::grp2gid(group) {
                Ok(g) => Some(g),
                _ => {
                    return Err(USimpleError::new(
                        1,
                        format!("invalid group: {}", group.quote()),
                    ))
                }
            }
        }
    };
    Ok((dest_gid, None, IfFrom::All))
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    chown_base(uu_app(), args, options::ARG_GROUP, parse_gid_and_uid, true)
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
                .action(ArgAction::Help)
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
            Arg::new(options::dereference::DEREFERENCE)
                .long(options::dereference::DEREFERENCE)
                .action(ArgAction::SetTrue),
        )
        .arg(
           Arg::new(options::dereference::NO_DEREFERENCE)
               .short('h')
               .long(options::dereference::NO_DEREFERENCE)
               .help(
                   "affect symbolic links instead of any referenced file (useful only on systems that can change the ownership of a symlink)",
               )
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
            Arg::new(options::RECURSIVE)
                .short('R')
                .long(options::RECURSIVE)
                .help("operate on files and directories recursively")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::traverse::TRAVERSE)
                .short(options::traverse::TRAVERSE.chars().next().unwrap())
                .help("if a command line argument is a symbolic link to a directory, traverse it")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::traverse::NO_TRAVERSE)
                .short(options::traverse::NO_TRAVERSE.chars().next().unwrap())
                .help("do not traverse any symbolic links (default)")
                .overrides_with_all([options::traverse::TRAVERSE, options::traverse::EVERY])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::traverse::EVERY)
                .short(options::traverse::EVERY.chars().next().unwrap())
                .help("traverse every symbolic link to a directory encountered")
                .action(ArgAction::SetTrue),
        )
}
