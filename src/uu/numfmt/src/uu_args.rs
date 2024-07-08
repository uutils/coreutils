// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{crate_version, Arg, ArgAction, Command};
use uucore::shortcut_value_parser::ShortcutValueParser;
use uucore::{format_usage, help_about, help_section, help_usage};

const ABOUT: &str = help_about!("numfmt.md");
const AFTER_HELP: &str = help_section!("after help", "numfmt.md");
const USAGE: &str = help_usage!("numfmt.md");

pub mod options {
    pub const DELIMITER: &str = "delimiter";
    pub const FIELD: &str = "field";
    pub const FIELD_DEFAULT: &str = "1";
    pub const FORMAT: &str = "format";
    pub const FROM: &str = "from";
    pub const FROM_DEFAULT: &str = "none";
    pub const FROM_UNIT: &str = "from-unit";
    pub const FROM_UNIT_DEFAULT: &str = "1";
    pub const HEADER: &str = "header";
    pub const HEADER_DEFAULT: &str = "1";
    pub const INVALID: &str = "invalid";
    pub const NUMBER: &str = "NUMBER";
    pub const PADDING: &str = "padding";
    pub const ROUND: &str = "round";
    pub const SUFFIX: &str = "suffix";
    pub const TO: &str = "to";
    pub const TO_DEFAULT: &str = "none";
    pub const TO_UNIT: &str = "to-unit";
    pub const TO_UNIT_DEFAULT: &str = "1";
}

#[allow(clippy::too_many_lines)]
pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .after_help(AFTER_HELP)
        .override_usage(format_usage(USAGE))
        .allow_negative_numbers(true)
        .infer_long_args(true)
        .arg(
            Arg::new(options::DELIMITER)
                .short('d')
                .long(options::DELIMITER)
                .value_name("X")
                .help("use X instead of whitespace for field delimiter"),
        )
        .arg(
            Arg::new(options::FIELD)
                .long(options::FIELD)
                .help("replace the numbers in these input fields; see FIELDS below")
                .value_name("FIELDS")
                .allow_hyphen_values(true)
                .default_value(options::FIELD_DEFAULT),
        )
        .arg(
            Arg::new(options::FORMAT)
                .long(options::FORMAT)
                .help("use printf style floating-point FORMAT; see FORMAT below for details")
                .value_name("FORMAT")
                .allow_hyphen_values(true),
        )
        .arg(
            Arg::new(options::FROM)
                .long(options::FROM)
                .help("auto-scale input numbers to UNITs; see UNIT below")
                .value_name("UNIT")
                .default_value(options::FROM_DEFAULT),
        )
        .arg(
            Arg::new(options::FROM_UNIT)
                .long(options::FROM_UNIT)
                .help("specify the input unit size")
                .value_name("N")
                .default_value(options::FROM_UNIT_DEFAULT),
        )
        .arg(
            Arg::new(options::TO)
                .long(options::TO)
                .help("auto-scale output numbers to UNITs; see UNIT below")
                .value_name("UNIT")
                .default_value(options::TO_DEFAULT),
        )
        .arg(
            Arg::new(options::TO_UNIT)
                .long(options::TO_UNIT)
                .help("the output unit size")
                .value_name("N")
                .default_value(options::TO_UNIT_DEFAULT),
        )
        .arg(
            Arg::new(options::PADDING)
                .long(options::PADDING)
                .help(
                    "pad the output to N characters; positive N will \
                     right-align; negative N will left-align; padding is \
                     ignored if the output is wider than N; the default is \
                     to automatically pad if a whitespace is found",
                )
                .value_name("N"),
        )
        .arg(
            Arg::new(options::HEADER)
                .long(options::HEADER)
                .help(
                    "print (without converting) the first N header lines; \
                     N defaults to 1 if not specified",
                )
                .num_args(..=1)
                .value_name("N")
                .default_missing_value(options::HEADER_DEFAULT)
                .hide_default_value(true),
        )
        .arg(
            Arg::new(options::ROUND)
                .long(options::ROUND)
                .help("use METHOD for rounding when scaling")
                .value_name("METHOD")
                .default_value("from-zero")
                .value_parser(ShortcutValueParser::new([
                    "up",
                    "down",
                    "from-zero",
                    "towards-zero",
                    "nearest",
                ])),
        )
        .arg(
            Arg::new(options::SUFFIX)
                .long(options::SUFFIX)
                .help(
                    "print SUFFIX after each formatted number, and accept \
                    inputs optionally ending with SUFFIX",
                )
                .value_name("SUFFIX"),
        )
        .arg(
            Arg::new(options::INVALID)
                .long(options::INVALID)
                .help("set the failure mode for invalid input")
                .default_value("abort")
                .value_parser(["abort", "fail", "warn", "ignore"])
                .value_name("INVALID"),
        )
        .arg(
            Arg::new(options::NUMBER)
                .hide(true)
                .action(ArgAction::Append),
        )
}
