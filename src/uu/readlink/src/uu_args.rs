// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{crate_version, Arg, ArgAction, Command};
use uucore::{format_usage, help_about, help_usage};

const ABOUT: &str = help_about!("readlink.md");
const USAGE: &str = help_usage!("readlink.md");

pub mod options {
    pub const OPT_CANONICALIZE: &str = "canonicalize";
    pub const OPT_CANONICALIZE_MISSING: &str = "canonicalize-missing";
    pub const OPT_CANONICALIZE_EXISTING: &str = "canonicalize-existing";
    pub const OPT_NO_NEWLINE: &str = "no-newline";
    pub const OPT_QUIET: &str = "quiet";
    pub const OPT_SILENT: &str = "silent";
    pub const OPT_VERBOSE: &str = "verbose";
    pub const OPT_ZERO: &str = "zero";

    pub const ARG_FILES: &str = "files";
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::OPT_CANONICALIZE)
                .short('f')
                .long(options::OPT_CANONICALIZE)
                .help(
                    "canonicalize by following every symlink in every component of the \
                     given name recursively; all but the last component must exist",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_CANONICALIZE_EXISTING)
                .short('e')
                .long("canonicalize-existing")
                .help(
                    "canonicalize by following every symlink in every component of the \
                     given name recursively, all components must exist",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_CANONICALIZE_MISSING)
                .short('m')
                .long(options::OPT_CANONICALIZE_MISSING)
                .help(
                    "canonicalize by following every symlink in every component of the \
                     given name recursively, without requirements on components existence",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_NO_NEWLINE)
                .short('n')
                .long(options::OPT_NO_NEWLINE)
                .help("do not output the trailing delimiter")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_QUIET)
                .short('q')
                .long(options::OPT_QUIET)
                .help("suppress most error messages")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_SILENT)
                .short('s')
                .long(options::OPT_SILENT)
                .help("suppress most error messages")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_VERBOSE)
                .short('v')
                .long(options::OPT_VERBOSE)
                .help("report error message")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_ZERO)
                .short('z')
                .long(options::OPT_ZERO)
                .help("separate output with NUL rather than newline")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::ARG_FILES)
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::AnyPath),
        )
}
