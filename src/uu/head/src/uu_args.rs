// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{crate_version, Arg, ArgAction, Command};
use uucore::{format_usage, help_about, help_usage};

const ABOUT: &str = help_about!("head.md");
const USAGE: &str = help_usage!("head.md");

pub mod options {
    pub const BYTES_NAME: &str = "BYTES";
    pub const LINES_NAME: &str = "LINES";
    pub const QUIET_NAME: &str = "QUIET";
    pub const VERBOSE_NAME: &str = "VERBOSE";
    pub const ZERO_NAME: &str = "ZERO";
    pub const FILES_NAME: &str = "FILE";
    pub const PRESUME_INPUT_PIPE: &str = "-PRESUME-INPUT-PIPE";
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::BYTES_NAME)
                .short('c')
                .long("bytes")
                .value_name("[-]NUM")
                .help(
                    "\
                     print the first NUM bytes of each file;\n\
                     with the leading '-', print all but the last\n\
                     NUM bytes of each file\
                     ",
                )
                .overrides_with_all([options::BYTES_NAME, options::LINES_NAME])
                .allow_hyphen_values(true),
        )
        .arg(
            Arg::new(options::LINES_NAME)
                .short('n')
                .long("lines")
                .value_name("[-]NUM")
                .help(
                    "\
                     print the first NUM lines instead of the first 10;\n\
                     with the leading '-', print all but the last\n\
                     NUM lines of each file\
                     ",
                )
                .overrides_with_all([options::LINES_NAME, options::BYTES_NAME])
                .allow_hyphen_values(true),
        )
        .arg(
            Arg::new(options::QUIET_NAME)
                .short('q')
                .long("quiet")
                .visible_alias("silent")
                .help("never print headers giving file names")
                .overrides_with_all([options::VERBOSE_NAME, options::QUIET_NAME])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::VERBOSE_NAME)
                .short('v')
                .long("verbose")
                .help("always print headers giving file names")
                .overrides_with_all([options::QUIET_NAME, options::VERBOSE_NAME])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::PRESUME_INPUT_PIPE)
                .long("presume-input-pipe")
                .alias("-presume-input-pipe")
                .hide(true)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::ZERO_NAME)
                .short('z')
                .long("zero-terminated")
                .help("line delimiter is NUL, not newline")
                .overrides_with(options::ZERO_NAME)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::FILES_NAME)
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::FilePath),
        )
}
