// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{builder::NonEmptyStringValueParser, crate_version, Arg, ArgAction, Command};
use uucore::{format_usage, help_about, help_usage};

static ABOUT: &str = help_about!("realpath.md");
const USAGE: &str = help_usage!("realpath.md");

pub mod options {

    pub static OPT_QUIET: &str = "quiet";
    pub static OPT_STRIP: &str = "strip";
    pub static OPT_ZERO: &str = "zero";
    pub static OPT_PHYSICAL: &str = "physical";
    pub static OPT_LOGICAL: &str = "logical";
    pub const OPT_CANONICALIZE_MISSING: &str = "canonicalize-missing";
    pub const OPT_CANONICALIZE_EXISTING: &str = "canonicalize-existing";
    pub const OPT_RELATIVE_TO: &str = "relative-to";
    pub const OPT_RELATIVE_BASE: &str = "relative-base";

    pub static ARG_FILES: &str = "files";
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::OPT_QUIET)
                .short('q')
                .long(options::OPT_QUIET)
                .help("Do not print warnings for invalid paths")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_STRIP)
                .short('s')
                .long(options::OPT_STRIP)
                .visible_alias("no-symlinks")
                .help("Only strip '.' and '..' components, but don't resolve symbolic links")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_ZERO)
                .short('z')
                .long(options::OPT_ZERO)
                .help("Separate output filenames with \\0 rather than newline")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_LOGICAL)
                .short('L')
                .long(options::OPT_LOGICAL)
                .help("resolve '..' components before symlinks")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_PHYSICAL)
                .short('P')
                .long(options::OPT_PHYSICAL)
                .overrides_with_all([options::OPT_STRIP, options::OPT_LOGICAL])
                .help("resolve symlinks as encountered (default)")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_CANONICALIZE_EXISTING)
                .short('e')
                .long(options::OPT_CANONICALIZE_EXISTING)
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
            Arg::new(options::OPT_RELATIVE_TO)
                .long(options::OPT_RELATIVE_TO)
                .value_name("DIR")
                .value_parser(NonEmptyStringValueParser::new())
                .help("print the resolved path relative to DIR"),
        )
        .arg(
            Arg::new(options::OPT_RELATIVE_BASE)
                .long(options::OPT_RELATIVE_BASE)
                .value_name("DIR")
                .value_parser(NonEmptyStringValueParser::new())
                .help("print absolute paths unless paths below DIR"),
        )
        .arg(
            Arg::new(options::ARG_FILES)
                .action(ArgAction::Append)
                .required(true)
                .value_parser(NonEmptyStringValueParser::new())
                .value_hint(clap::ValueHint::AnyPath),
        )
}
