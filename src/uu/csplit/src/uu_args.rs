// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{crate_version, Arg, ArgAction, Command};
use uucore::{format_usage, help_about, help_section, help_usage};

const ABOUT: &str = help_about!("csplit.md");
const AFTER_HELP: &str = help_section!("after help", "csplit.md");
const USAGE: &str = help_usage!("csplit.md");

pub mod options {
    pub const SUFFIX_FORMAT: &str = "suffix-format";
    pub const SUPPRESS_MATCHED: &str = "suppress-matched";
    pub const DIGITS: &str = "digits";
    pub const PREFIX: &str = "prefix";
    pub const KEEP_FILES: &str = "keep-files";
    pub const QUIET: &str = "quiet";
    pub const ELIDE_EMPTY_FILES: &str = "elide-empty-files";
    pub const FILE: &str = "file";
    pub const PATTERN: &str = "pattern";
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .args_override_self(true)
        .infer_long_args(true)
        .arg(
            Arg::new(options::SUFFIX_FORMAT)
                .short('b')
                .long(options::SUFFIX_FORMAT)
                .value_name("FORMAT")
                .help("use sprintf FORMAT instead of %02d"),
        )
        .arg(
            Arg::new(options::PREFIX)
                .short('f')
                .long(options::PREFIX)
                .value_name("PREFIX")
                .help("use PREFIX instead of 'xx'"),
        )
        .arg(
            Arg::new(options::KEEP_FILES)
                .short('k')
                .long(options::KEEP_FILES)
                .help("do not remove output files on errors")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SUPPRESS_MATCHED)
                .long(options::SUPPRESS_MATCHED)
                .help("suppress the lines matching PATTERN")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::DIGITS)
                .short('n')
                .long(options::DIGITS)
                .value_name("DIGITS")
                .help("use specified number of digits instead of 2"),
        )
        .arg(
            Arg::new(options::QUIET)
                .short('s')
                .long(options::QUIET)
                .visible_alias("silent")
                .help("do not print counts of output file sizes")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::ELIDE_EMPTY_FILES)
                .short('z')
                .long(options::ELIDE_EMPTY_FILES)
                .help("remove empty output files")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::FILE)
                .hide(true)
                .required(true)
                .value_hint(clap::ValueHint::FilePath),
        )
        .arg(
            Arg::new(options::PATTERN)
                .hide(true)
                .action(clap::ArgAction::Append)
                .required(true),
        )
        .after_help(AFTER_HELP)
}
