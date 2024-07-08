// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{builder::ValueParser, crate_version, Arg, ArgAction, Command};

use uucore::{format_usage, help_about, help_usage};

const ABOUT: &str = help_about!("chcon.md");
const USAGE: &str = help_usage!("chcon.md");

pub mod options {
    pub static HELP: &str = "help";
    pub static VERBOSE: &str = "verbose";

    pub static REFERENCE: &str = "reference";

    pub static USER: &str = "user";
    pub static ROLE: &str = "role";
    pub static TYPE: &str = "type";
    pub static RANGE: &str = "range";

    pub static RECURSIVE: &str = "recursive";

    pub mod sym_links {
        pub static FOLLOW_ARG_DIR_SYM_LINK: &str = "follow-arg-dir-sym-link";
        pub static FOLLOW_DIR_SYM_LINKS: &str = "follow-dir-sym-links";
        pub static NO_FOLLOW_SYM_LINKS: &str = "no-follow-sym-links";
    }

    pub mod dereference {
        pub static DEREFERENCE: &str = "dereference";
        pub static NO_DEREFERENCE: &str = "no-dereference";
    }

    pub mod preserve_root {
        pub static PRESERVE_ROOT: &str = "preserve-root";
        pub static NO_PRESERVE_ROOT: &str = "no-preserve-root";
    }
}

#[allow(clippy::too_many_lines)]
pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .disable_help_flag(true)
        .args_override_self(true)
        .arg(
            Arg::new(options::HELP)
                .long(options::HELP)
                .help("Print help information.")
                .action(ArgAction::Help),
        )
        .arg(
            Arg::new(options::dereference::DEREFERENCE)
                .long(options::dereference::DEREFERENCE)
                .overrides_with(options::dereference::NO_DEREFERENCE)
                .help(
                    "Affect the referent of each symbolic link (this is the default), \
                     rather than the symbolic link itself.",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::dereference::NO_DEREFERENCE)
                .short('h')
                .long(options::dereference::NO_DEREFERENCE)
                .help("Affect symbolic links instead of any referenced file.")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::preserve_root::PRESERVE_ROOT)
                .long(options::preserve_root::PRESERVE_ROOT)
                .overrides_with(options::preserve_root::NO_PRESERVE_ROOT)
                .help("Fail to operate recursively on '/'.")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::preserve_root::NO_PRESERVE_ROOT)
                .long(options::preserve_root::NO_PRESERVE_ROOT)
                .help("Do not treat '/' specially (the default).")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::REFERENCE)
                .long(options::REFERENCE)
                .value_name("RFILE")
                .value_hint(clap::ValueHint::FilePath)
                .conflicts_with_all([options::USER, options::ROLE, options::TYPE, options::RANGE])
                .help(
                    "Use security context of RFILE, rather than specifying \
                     a CONTEXT value.",
                )
                .value_parser(ValueParser::os_string()),
        )
        .arg(
            Arg::new(options::USER)
                .short('u')
                .long(options::USER)
                .value_name("USER")
                .value_hint(clap::ValueHint::Username)
                .help("Set user USER in the target security context.")
                .value_parser(ValueParser::os_string()),
        )
        .arg(
            Arg::new(options::ROLE)
                .short('r')
                .long(options::ROLE)
                .value_name("ROLE")
                .help("Set role ROLE in the target security context.")
                .value_parser(ValueParser::os_string()),
        )
        .arg(
            Arg::new(options::TYPE)
                .short('t')
                .long(options::TYPE)
                .value_name("TYPE")
                .help("Set type TYPE in the target security context.")
                .value_parser(ValueParser::os_string()),
        )
        .arg(
            Arg::new(options::RANGE)
                .short('l')
                .long(options::RANGE)
                .value_name("RANGE")
                .help("Set range RANGE in the target security context.")
                .value_parser(ValueParser::os_string()),
        )
        .arg(
            Arg::new(options::RECURSIVE)
                .short('R')
                .long(options::RECURSIVE)
                .help("Operate on files and directories recursively.")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::sym_links::FOLLOW_ARG_DIR_SYM_LINK)
                .short('H')
                .requires(options::RECURSIVE)
                .overrides_with_all([
                    options::sym_links::FOLLOW_DIR_SYM_LINKS,
                    options::sym_links::NO_FOLLOW_SYM_LINKS,
                ])
                .help(
                    "If a command line argument is a symbolic link to a directory, \
                     traverse it. Only valid when -R is specified.",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::sym_links::FOLLOW_DIR_SYM_LINKS)
                .short('L')
                .requires(options::RECURSIVE)
                .overrides_with_all([
                    options::sym_links::FOLLOW_ARG_DIR_SYM_LINK,
                    options::sym_links::NO_FOLLOW_SYM_LINKS,
                ])
                .help(
                    "Traverse every symbolic link to a directory encountered. \
                     Only valid when -R is specified.",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::sym_links::NO_FOLLOW_SYM_LINKS)
                .short('P')
                .requires(options::RECURSIVE)
                .overrides_with_all([
                    options::sym_links::FOLLOW_ARG_DIR_SYM_LINK,
                    options::sym_links::FOLLOW_DIR_SYM_LINKS,
                ])
                .help(
                    "Do not traverse any symbolic links (default). \
                     Only valid when -R is specified.",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::VERBOSE)
                .short('v')
                .long(options::VERBOSE)
                .help("Output a diagnostic for every file processed.")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("FILE")
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::FilePath)
                .num_args(1..)
                .value_parser(ValueParser::os_string()),
        )
}
