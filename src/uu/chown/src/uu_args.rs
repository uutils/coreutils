// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore RFILE's RFILE

use clap::{crate_version, Arg, ArgAction, Command};
use uucore::perms::options;
use uucore::{format_usage, help_about, help_usage};

static ABOUT: &str = help_about!("chown.md");
const USAGE: &str = help_usage!("chown.md");

#[allow(clippy::too_many_lines)]
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
                .overrides_with_all([options::traverse::EVERY, options::traverse::NO_TRAVERSE])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::traverse::EVERY)
                .short(options::traverse::EVERY.chars().next().unwrap())
                .help("traverse every symbolic link to a directory encountered")
                .overrides_with_all([options::traverse::TRAVERSE, options::traverse::NO_TRAVERSE])
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
            Arg::new(options::verbosity::VERBOSE)
                .long(options::verbosity::VERBOSE)
                .short('v')
                .help("output a diagnostic for every file processed")
                .action(ArgAction::SetTrue),
        )
}
