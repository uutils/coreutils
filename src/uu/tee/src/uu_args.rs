// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{builder::PossibleValue, crate_version, Arg, ArgAction, Command};
use uucore::{
    format_usage, help_about, help_section, help_usage, shortcut_value_parser::ShortcutValueParser,
};

const ABOUT: &str = help_about!("tee.md");
const USAGE: &str = help_usage!("tee.md");
const AFTER_HELP: &str = help_section!("after help", "tee.md");

pub mod options {
    pub const APPEND: &str = "append";
    pub const IGNORE_INTERRUPTS: &str = "ignore-interrupts";
    pub const FILE: &str = "file";
    pub const IGNORE_PIPE_ERRORS: &str = "ignore-pipe-errors";
    pub const OUTPUT_ERROR: &str = "output-error";
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .after_help(AFTER_HELP)
        .infer_long_args(true)
        // Since we use value-specific help texts for "--output-error", clap's "short help" and "long help" differ.
        // However, this is something that the GNU tests explicitly test for, so we *always* show the long help instead.
        .disable_help_flag(true)
        .arg(
            Arg::new("--help")
                .short('h')
                .long("help")
                .help("Print help")
                .action(ArgAction::HelpLong)
        )
        .arg(
            Arg::new(options::APPEND)
                .long(options::APPEND)
                .short('a')
                .help("append to the given FILEs, do not overwrite")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::IGNORE_INTERRUPTS)
                .long(options::IGNORE_INTERRUPTS)
                .short('i')
                .help("ignore interrupt signals (ignored on non-Unix platforms)")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::FILE)
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::FilePath),
        )
        .arg(
            Arg::new(options::IGNORE_PIPE_ERRORS)
                .short('p')
                .help("set write error behavior (ignored on non-Unix platforms)")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OUTPUT_ERROR)
                .long(options::OUTPUT_ERROR)
                .require_equals(true)
                .num_args(0..=1)
                .value_parser(ShortcutValueParser::new([
                    PossibleValue::new("warn")
                        .help("produce warnings for errors writing to any output"),
                    PossibleValue::new("warn-nopipe")
                        .help("produce warnings for errors that are not pipe errors (ignored on non-unix platforms)"),
                    PossibleValue::new("exit").help("exit on write errors to any output"),
                    PossibleValue::new("exit-nopipe")
                        .help("exit on write errors to any output that are not pipe errors (equivalent to exit on non-unix platforms)"),
                ]))
                .help("set write error behavior")
                .conflicts_with(options::IGNORE_PIPE_ERRORS),
        )
}
