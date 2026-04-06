// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{Arg, ArgAction, Command, builder::ValueParser};
use uucore::translate;

use uucore::{format_usage, parser::shortcut_value_parser::ShortcutValueParser};

pub mod options {
    pub static BYTES: &str = "bytes";
    pub static CHAR: &str = "chars";
    pub static FILES0_FROM: &str = "files0-from";
    pub static LINES: &str = "lines";
    pub static MAX_LINE_LENGTH: &str = "max-line-length";
    pub static TOTAL: &str = "total";
    pub static WORDS: &str = "words";
    pub static DEBUG: &str = "debug";
}
pub static ARG_FILES: &str = "files";

pub fn uu_app() -> Command {
    Command::new("wc")
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .about(translate!("wc-about"))
        .override_usage(format_usage(&translate!("wc-usage")))
        .infer_long_args(true)
        .args_override_self(true)
        .arg(
            Arg::new(options::BYTES)
                .short('c')
                .long(options::BYTES)
                .help(translate!("wc-help-bytes"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::CHAR)
                .short('m')
                .long(options::CHAR)
                .help(translate!("wc-help-chars"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::FILES0_FROM)
                .long(options::FILES0_FROM)
                .value_name("F")
                .help(translate!("wc-help-files0-from"))
                .value_parser(ValueParser::os_string())
                .value_hint(clap::ValueHint::FilePath),
        )
        .arg(
            Arg::new(options::LINES)
                .short('l')
                .long(options::LINES)
                .help(translate!("wc-help-lines"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::MAX_LINE_LENGTH)
                .short('L')
                .long(options::MAX_LINE_LENGTH)
                .help(translate!("wc-help-max-line-length"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::TOTAL)
                .long(options::TOTAL)
                .value_parser(ShortcutValueParser::new([
                    "auto", "always", "only", "never",
                ]))
                .value_name("WHEN")
                .hide_possible_values(true)
                .help(translate!("wc-help-total")),
        )
        .arg(
            Arg::new(options::WORDS)
                .short('w')
                .long(options::WORDS)
                .help(translate!("wc-help-words"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::DEBUG)
                .long(options::DEBUG)
                .action(ArgAction::SetTrue)
                .hide(true),
        )
        .arg(
            Arg::new(ARG_FILES)
                .action(ArgAction::Append)
                .value_parser(ValueParser::os_string())
                .value_hint(clap::ValueHint::FilePath),
        )
}
