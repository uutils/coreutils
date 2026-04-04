// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore nopipe

use clap::{Arg, ArgAction, Command, builder::PossibleValue};
use std::ffi::OsString;
use uucore::parser::shortcut_value_parser::ShortcutValueParser;
pub use uucore::{format_usage, translate};

pub mod options {
    pub const APPEND: &str = "append";
    pub const IGNORE_INTERRUPTS: &str = "ignore-interrupts";
    pub const FILE: &str = "file";
    pub const IGNORE_PIPE_ERRORS: &str = "ignore-pipe-errors";
    pub const OUTPUT_ERROR: &str = "output-error";
}

#[derive(Clone, Debug)]
pub enum OutputErrorMode {
    /// Diagnose write error on any output
    Warn,
    /// Diagnose write error on any output that is not a pipe
    WarnNoPipe,
    /// Exit upon write error on any output
    Exit,
    /// Exit upon write error on any output that is not a pipe
    ExitNoPipe,
}

#[allow(dead_code)]
pub struct Options {
    pub append: bool,
    pub ignore_interrupts: bool,
    pub ignore_pipe_errors: bool,
    pub files: Vec<OsString>,
    pub output_error: Option<OutputErrorMode>,
}

pub fn uu_app() -> Command {
    Command::new("tee")
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template("tee"))
        .about(translate!("tee-about"))
        .override_usage(format_usage(&translate!("tee-usage")))
        .after_help(translate!("tee-after-help"))
        .infer_long_args(true)
        // Since we use value-specific help texts for "--output-error", clap's "short help" and "long help" differ.
        // However, this is something that the GNU tests explicitly test for, so we *always* show the long help instead.
        .disable_help_flag(true)
        .arg(
            Arg::new("--help")
                .short('h')
                .long("help")
                .help(translate!("tee-help-help"))
                .action(ArgAction::HelpLong),
        )
        .arg(
            Arg::new(options::APPEND)
                .long(options::APPEND)
                .short('a')
                .help(translate!("tee-help-append"))
                .action(ArgAction::SetTrue)
                .overrides_with(options::APPEND),
        )
        .arg(
            Arg::new(options::IGNORE_INTERRUPTS)
                .long(options::IGNORE_INTERRUPTS)
                .short('i')
                .help(translate!("tee-help-ignore-interrupts"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::FILE)
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::FilePath)
                .value_parser(clap::value_parser!(OsString)),
        )
        .arg(
            Arg::new(options::IGNORE_PIPE_ERRORS)
                .short('p')
                .help(translate!("tee-help-ignore-pipe-errors"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OUTPUT_ERROR)
                .long(options::OUTPUT_ERROR)
                .require_equals(true)
                .num_args(0..=1)
                .default_missing_value("warn-nopipe")
                .value_parser(ShortcutValueParser::new([
                    PossibleValue::new("warn").help(translate!("tee-help-output-error-warn")),
                    PossibleValue::new("warn-nopipe")
                        .help(translate!("tee-help-output-error-warn-nopipe")),
                    PossibleValue::new("exit").help(translate!("tee-help-output-error-exit")),
                    PossibleValue::new("exit-nopipe")
                        .help(translate!("tee-help-output-error-exit-nopipe")),
                ]))
                .help(translate!("tee-help-output-error")),
        )
}
