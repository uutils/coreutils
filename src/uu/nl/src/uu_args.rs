// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{crate_version, Arg, ArgAction, Command};

use uucore::{format_usage, help_about, help_section, help_usage};

const ABOUT: &str = help_about!("nl.md");
const AFTER_HELP: &str = help_section!("after help", "nl.md");
const USAGE: &str = help_usage!("nl.md");

pub mod options {
    pub const HELP: &str = "help";
    pub const FILE: &str = "file";
    pub const BODY_NUMBERING: &str = "body-numbering";
    pub const SECTION_DELIMITER: &str = "section-delimiter";
    pub const FOOTER_NUMBERING: &str = "footer-numbering";
    pub const HEADER_NUMBERING: &str = "header-numbering";
    pub const LINE_INCREMENT: &str = "line-increment";
    pub const JOIN_BLANK_LINES: &str = "join-blank-lines";
    pub const NUMBER_FORMAT: &str = "number-format";
    pub const NO_RENUMBER: &str = "no-renumber";
    pub const NUMBER_SEPARATOR: &str = "number-separator";
    pub const STARTING_LINE_NUMBER: &str = "starting-line-number";
    pub const NUMBER_WIDTH: &str = "number-width";
}

#[allow(clippy::too_many_lines)]
pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .about(ABOUT)
        .version(crate_version!())
        .override_usage(format_usage(USAGE))
        .after_help(AFTER_HELP)
        .infer_long_args(true)
        .disable_help_flag(true)
        .arg(
            Arg::new(options::HELP)
                .long(options::HELP)
                .help("Print help information.")
                .action(ArgAction::Help),
        )
        .arg(
            Arg::new(options::FILE)
                .hide(true)
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::FilePath),
        )
        .arg(
            Arg::new(options::BODY_NUMBERING)
                .short('b')
                .long(options::BODY_NUMBERING)
                .help("use STYLE for numbering body lines")
                .value_name("STYLE"),
        )
        .arg(
            Arg::new(options::SECTION_DELIMITER)
                .short('d')
                .long(options::SECTION_DELIMITER)
                .help("use CC for separating logical pages")
                .value_name("CC"),
        )
        .arg(
            Arg::new(options::FOOTER_NUMBERING)
                .short('f')
                .long(options::FOOTER_NUMBERING)
                .help("use STYLE for numbering footer lines")
                .value_name("STYLE"),
        )
        .arg(
            Arg::new(options::HEADER_NUMBERING)
                .short('h')
                .long(options::HEADER_NUMBERING)
                .help("use STYLE for numbering header lines")
                .value_name("STYLE"),
        )
        .arg(
            Arg::new(options::LINE_INCREMENT)
                .short('i')
                .long(options::LINE_INCREMENT)
                .help("line number increment at each line")
                .value_name("NUMBER")
                .value_parser(clap::value_parser!(i64)),
        )
        .arg(
            Arg::new(options::JOIN_BLANK_LINES)
                .short('l')
                .long(options::JOIN_BLANK_LINES)
                .help("group of NUMBER empty lines counted as one")
                .value_name("NUMBER")
                .value_parser(clap::value_parser!(u64)),
        )
        .arg(
            Arg::new(options::NUMBER_FORMAT)
                .short('n')
                .long(options::NUMBER_FORMAT)
                .help("insert line numbers according to FORMAT")
                .value_name("FORMAT")
                .value_parser(["ln", "rn", "rz"]),
        )
        .arg(
            Arg::new(options::NO_RENUMBER)
                .short('p')
                .long(options::NO_RENUMBER)
                .help("do not reset line numbers at logical pages")
                .action(ArgAction::SetFalse),
        )
        .arg(
            Arg::new(options::NUMBER_SEPARATOR)
                .short('s')
                .long(options::NUMBER_SEPARATOR)
                .help("add STRING after (possible) line number")
                .value_name("STRING"),
        )
        .arg(
            Arg::new(options::STARTING_LINE_NUMBER)
                .short('v')
                .long(options::STARTING_LINE_NUMBER)
                .help("first line number on each logical page")
                .value_name("NUMBER")
                .value_parser(clap::value_parser!(i64)),
        )
        .arg(
            Arg::new(options::NUMBER_WIDTH)
                .short('w')
                .long(options::NUMBER_WIDTH)
                .help("use NUMBER columns for line numbers")
                .value_name("NUMBER")
                .value_parser(clap::value_parser!(usize)),
        )
}
