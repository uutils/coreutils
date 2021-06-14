use clap::{crate_version, App, Arg};

const USAGE: &str = "nl [OPTION]... [FILE]...";

pub mod options {
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

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .usage(USAGE)
        .arg(Arg::with_name(options::FILE).hidden(true).multiple(true))
        .arg(
            Arg::with_name(options::BODY_NUMBERING)
                .short("b")
                .long(options::BODY_NUMBERING)
                .help("use STYLE for numbering body lines")
                .value_name("SYNTAX"),
        )
        .arg(
            Arg::with_name(options::SECTION_DELIMITER)
                .short("d")
                .long(options::SECTION_DELIMITER)
                .help("use CC for separating logical pages")
                .value_name("CC"),
        )
        .arg(
            Arg::with_name(options::FOOTER_NUMBERING)
                .short("f")
                .long(options::FOOTER_NUMBERING)
                .help("use STYLE for numbering footer lines")
                .value_name("STYLE"),
        )
        .arg(
            Arg::with_name(options::HEADER_NUMBERING)
                .short("h")
                .long(options::HEADER_NUMBERING)
                .help("use STYLE for numbering header lines")
                .value_name("STYLE"),
        )
        .arg(
            Arg::with_name(options::LINE_INCREMENT)
                .short("i")
                .long(options::LINE_INCREMENT)
                .help("line number increment at each line")
                .value_name("NUMBER"),
        )
        .arg(
            Arg::with_name(options::JOIN_BLANK_LINES)
                .short("l")
                .long(options::JOIN_BLANK_LINES)
                .help("group of NUMBER empty lines counted as one")
                .value_name("NUMBER"),
        )
        .arg(
            Arg::with_name(options::NUMBER_FORMAT)
                .short("n")
                .long(options::NUMBER_FORMAT)
                .help("insert line numbers according to FORMAT")
                .value_name("FORMAT"),
        )
        .arg(
            Arg::with_name(options::NO_RENUMBER)
                .short("p")
                .long(options::NO_RENUMBER)
                .help("do not reset line numbers at logical pages"),
        )
        .arg(
            Arg::with_name(options::NUMBER_SEPARATOR)
                .short("s")
                .long(options::NUMBER_SEPARATOR)
                .help("add STRING after (possible) line number")
                .value_name("STRING"),
        )
        .arg(
            Arg::with_name(options::STARTING_LINE_NUMBER)
                .short("v")
                .long(options::STARTING_LINE_NUMBER)
                .help("first line number on each logical page")
                .value_name("NUMBER"),
        )
        .arg(
            Arg::with_name(options::NUMBER_WIDTH)
                .short("w")
                .long(options::NUMBER_WIDTH)
                .help("use NUMBER columns for line numbers")
                .value_name("NUMBER"),
        )
}
