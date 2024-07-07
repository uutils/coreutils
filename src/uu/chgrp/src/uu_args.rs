// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{crate_version, Arg, ArgAction, Command};
use uucore::perms::options;
use uucore::{format_usage, help_about, help_usage};

const ABOUT: &str = help_about!("chgrp.md");
const USAGE: &str = help_usage!("chgrp.md");

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
