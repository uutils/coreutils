// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{Arg, ArgAction, Command};
use std::ffi::OsString;
use uucore::format_usage;
use uucore::translate;

pub mod options {
    pub const BYTES: &str = "BYTES";
    pub const LINES: &str = "LINES";
    pub const QUIET: &str = "QUIET";
    pub const VERBOSE: &str = "VERBOSE";
    pub const ZERO: &str = "ZERO";
    pub const FILES: &str = "FILE";
    pub const PRESUME_INPUT_PIPE: &str = "-PRESUME-INPUT-PIPE";
}

pub fn uu_app() -> Command {
    Command::new("head")
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template("head"))
        .about(translate!("head-about"))
        .override_usage(format_usage(&translate!("head-usage")))
        .infer_long_args(true)
        .arg(
            Arg::new(options::BYTES)
                .short('c')
                .long("bytes")
                .value_name("[-]NUM")
                .help(translate!("head-help-bytes"))
                .overrides_with_all([options::BYTES, options::LINES])
                .allow_hyphen_values(true),
        )
        .arg(
            Arg::new(options::LINES)
                .short('n')
                .long("lines")
                .value_name("[-]NUM")
                .help(translate!("head-help-lines"))
                .overrides_with_all([options::LINES, options::BYTES])
                .allow_hyphen_values(true),
        )
        .arg(
            Arg::new(options::QUIET)
                .short('q')
                .long("quiet")
                .visible_alias("silent")
                .help(translate!("head-help-quiet"))
                .overrides_with_all([options::VERBOSE, options::QUIET])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::VERBOSE)
                .short('v')
                .long("verbose")
                .help(translate!("head-help-verbose"))
                .overrides_with_all([options::QUIET, options::VERBOSE])
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
            Arg::new(options::ZERO)
                .short('z')
                .long("zero-terminated")
                .help(translate!("head-help-zero-terminated"))
                .overrides_with(options::ZERO)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::FILES)
                .action(ArgAction::Append)
                .value_parser(clap::value_parser!(OsString))
                .value_hint(clap::ValueHint::FilePath),
        )
}
