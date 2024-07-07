// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{builder::ValueParser, crate_version, Arg, ArgAction, Command};

use uucore::shortcut_value_parser::ShortcutValueParser;
use uucore::{format_usage, help_about, help_section, help_usage};

const ABOUT: &str = help_about!("uniq.md");
const USAGE: &str = help_usage!("uniq.md");
const AFTER_HELP: &str = help_section!("after help", "uniq.md");

pub mod options {
    pub static ALL_REPEATED: &str = "all-repeated";
    pub static CHECK_CHARS: &str = "check-chars";
    pub static COUNT: &str = "count";
    pub static IGNORE_CASE: &str = "ignore-case";
    pub static REPEATED: &str = "repeated";
    pub static SKIP_FIELDS: &str = "skip-fields";
    pub static SKIP_CHARS: &str = "skip-chars";
    pub static UNIQUE: &str = "unique";
    pub static ZERO_TERMINATED: &str = "zero-terminated";
    pub static GROUP: &str = "group";
}

pub static ARG_FILES: &str = "files";

#[allow(clippy::too_many_lines)]
pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .after_help(AFTER_HELP)
        .arg(
            Arg::new(options::ALL_REPEATED)
                .short('D')
                .long(options::ALL_REPEATED)
                .value_parser(ShortcutValueParser::new([
                    "none",
                    "prepend",
                    "separate"
                ]))
                .help("print all duplicate lines. Delimiting is done with blank lines. [default: none]")
                .value_name("delimit-method")
                .num_args(0..=1)
                .default_missing_value("none")
                .require_equals(true),
        )
        .arg(
            Arg::new(options::GROUP)
                .long(options::GROUP)
                .value_parser(ShortcutValueParser::new([
                    "separate",
                    "prepend",
                    "append",
                    "both",
                ]))
                .help("show all items, separating groups with an empty line. [default: separate]")
                .value_name("group-method")
                .num_args(0..=1)
                .default_missing_value("separate")
                .require_equals(true)
                .conflicts_with_all([
                    options::REPEATED,
                    options::ALL_REPEATED,
                    options::UNIQUE,
                    options::COUNT
                ]),
        )
        .arg(
            Arg::new(options::CHECK_CHARS)
                .short('w')
                .long(options::CHECK_CHARS)
                .help("compare no more than N characters in lines")
                .value_name("N"),
        )
        .arg(
            Arg::new(options::COUNT)
                .short('c')
                .long(options::COUNT)
                .help("prefix lines by the number of occurrences")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::IGNORE_CASE)
                .short('i')
                .long(options::IGNORE_CASE)
                .help("ignore differences in case when comparing")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::REPEATED)
                .short('d')
                .long(options::REPEATED)
                .help("only print duplicate lines")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SKIP_CHARS)
                .short('s')
                .long(options::SKIP_CHARS)
                .help("avoid comparing the first N characters")
                .value_name("N"),
        )
        .arg(
            Arg::new(options::SKIP_FIELDS)
                .short('f')
                .long(options::SKIP_FIELDS)
                .help("avoid comparing the first N fields")
                .value_name("N"),
        )
        .arg(
            Arg::new(options::UNIQUE)
                .short('u')
                .long(options::UNIQUE)
                .help("only print unique lines")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::ZERO_TERMINATED)
                .short('z')
                .long(options::ZERO_TERMINATED)
                .help("end lines with 0 byte, not newline")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(ARG_FILES)
                .action(ArgAction::Append)
                .value_parser(ValueParser::os_string())
                .num_args(0..=2)
                .hide(true)
                .value_hint(clap::ValueHint::FilePath),
        )
}
