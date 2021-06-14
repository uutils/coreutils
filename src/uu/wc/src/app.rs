use clap::{crate_version, App, Arg};

const ABOUT: &str = "Display newline, word, and byte counts for each FILE, and a total line if
more than one FILE is specified.";

pub mod options {
    pub const BYTES: &str = "bytes";
    pub const CHAR: &str = "chars";
    pub const LINES: &str = "lines";
    pub const MAX_LINE_LENGTH: &str = "max-line-length";
    pub const WORDS: &str = "words";
}

pub const ARG_FILES: &str = "files";

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .about(ABOUT)
        .arg(
            Arg::with_name(options::BYTES)
                .short("c")
                .long(options::BYTES)
                .help("print the byte counts"),
        )
        .arg(
            Arg::with_name(options::CHAR)
                .short("m")
                .long(options::CHAR)
                .help("print the character counts"),
        )
        .arg(
            Arg::with_name(options::LINES)
                .short("l")
                .long(options::LINES)
                .help("print the newline counts"),
        )
        .arg(
            Arg::with_name(options::MAX_LINE_LENGTH)
                .short("L")
                .long(options::MAX_LINE_LENGTH)
                .help("print the length of the longest line"),
        )
        .arg(
            Arg::with_name(options::WORDS)
                .short("w")
                .long(options::WORDS)
                .help("print the word counts"),
        )
        .arg(Arg::with_name(ARG_FILES).multiple(true).takes_value(true))
}
