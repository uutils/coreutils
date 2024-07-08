// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{builder::ValueParser, crate_version, Arg, ArgAction, Command};

use uucore::{format_usage, help_about, help_section, help_usage};

const USAGE: &str = help_usage!("cut.md");
const ABOUT: &str = help_about!("cut.md");
const AFTER_HELP: &str = help_section!("after help", "cut.md");

pub mod options {
    pub const BYTES: &str = "bytes";
    pub const CHARACTERS: &str = "characters";
    pub const DELIMITER: &str = "delimiter";
    pub const FIELDS: &str = "fields";
    pub const ZERO_TERMINATED: &str = "zero-terminated";
    pub const ONLY_DELIMITED: &str = "only-delimited";
    pub const OUTPUT_DELIMITER: &str = "output-delimiter";
    pub const WHITESPACE_DELIMITED: &str = "whitespace-delimited";
    pub const COMPLEMENT: &str = "complement";
    pub const FILE: &str = "file";
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .override_usage(format_usage(USAGE))
        .about(ABOUT)
        .after_help(AFTER_HELP)
        .infer_long_args(true)
        // While `args_override_self(true)` for some arguments, such as `-d`
        // and `--output-delimiter`, is consistent to the behavior of GNU cut,
        // arguments related to cutting mode, i.e. `-b`, `-c`, `-f`, should
        // cause an error when there is more than one of them, as described in
        // the manual of GNU cut: "Use one, and only one of -b, -c or -f".
        // `ArgAction::Append` is used on `-b`, `-c`, `-f` arguments, so that
        // the occurrences of those could be counted and be handled accordingly.
        .args_override_self(true)
        .arg(
            Arg::new(options::BYTES)
                .short('b')
                .long(options::BYTES)
                .help("filter byte columns from the input source")
                .allow_hyphen_values(true)
                .value_name("LIST")
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new(options::CHARACTERS)
                .short('c')
                .long(options::CHARACTERS)
                .help("alias for character mode")
                .allow_hyphen_values(true)
                .value_name("LIST")
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new(options::DELIMITER)
                .short('d')
                .long(options::DELIMITER)
                .value_parser(ValueParser::os_string())
                .help("specify the delimiter character that separates fields in the input source. Defaults to Tab.")
                .value_name("DELIM"),
        )
        .arg(
            Arg::new(options::WHITESPACE_DELIMITED)
                .short('w')
                .help("Use any number of whitespace (Space, Tab) to separate fields in the input source (FreeBSD extension).")
                .value_name("WHITESPACE")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::FIELDS)
                .short('f')
                .long(options::FIELDS)
                .help("filter field columns from the input source")
                .allow_hyphen_values(true)
                .value_name("LIST")
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new(options::COMPLEMENT)
                .long(options::COMPLEMENT)
                .help("invert the filter - instead of displaying only the filtered columns, display all but those columns")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::ONLY_DELIMITED)
                .short('s')
                .long(options::ONLY_DELIMITED)
                .help("in field mode, only print lines which contain the delimiter")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::ZERO_TERMINATED)
                .short('z')
                .long(options::ZERO_TERMINATED)
                .help("instead of filtering columns based on line, filter columns based on \\0 (NULL character)")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OUTPUT_DELIMITER)
                .long(options::OUTPUT_DELIMITER)
                .value_parser(ValueParser::os_string())
                .help("in field mode, replace the delimiter in output lines with this option's argument")
                .value_name("NEW_DELIM"),
        )
        .arg(
            Arg::new(options::FILE)
            .hide(true)
            .action(ArgAction::Append)
            .value_hint(clap::ValueHint::FilePath)
        )
}
