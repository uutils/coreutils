use clap::{crate_version, App, Arg};

#[allow(dead_code)]
pub mod options {
    pub const SILENT: &str = "silent";
    pub const LOGICAL: &str = "logical";
    pub const NO_PAUSE: &str = "no-pause";
    pub const PRINT_OVER: &str = "print-over";
    pub const CLEAN_PRINT: &str = "clean-print";
    pub const SQUEEZE: &str = "squeeze";
    pub const PLAIN: &str = "plain";
    pub const LINES: &str = "lines";
    pub const NUMBER: &str = "number";
    pub const PATTERN: &str = "pattern";
    pub const FROM_LINE: &str = "from-line";
    pub const FILES: &str = "files";
}

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .about("A file perusal filter for CRT viewing.")
        .version(crate_version!())
        .arg(
            Arg::with_name(options::SILENT)
                .short("d")
                .long(options::SILENT)
                .help("Display help instead of ringing bell"),
        )
        // The commented arguments below are unimplemented:
        /*
        .arg(
            Arg::with_name(options::LOGICAL)
                .short("f")
                .long(options::LOGICAL)
                .help("Count logical rather than screen lines"),
        )
        .arg(
            Arg::with_name(options::NO_PAUSE)
                .short("l")
                .long(options::NO_PAUSE)
                .help("Suppress pause after form feed"),
        )
        .arg(
            Arg::with_name(options::PRINT_OVER)
                .short("c")
                .long(options::PRINT_OVER)
                .help("Do not scroll, display text and clean line ends"),
        )
        .arg(
            Arg::with_name(options::CLEAN_PRINT)
                .short("p")
                .long(options::CLEAN_PRINT)
                .help("Do not scroll, clean screen and display text"),
        )
        .arg(
            Arg::with_name(options::SQUEEZE)
                .short("s")
                .long(options::SQUEEZE)
                .help("Squeeze multiple blank lines into one"),
        )
        .arg(
            Arg::with_name(options::PLAIN)
                .short("u")
                .long(options::PLAIN)
                .help("Suppress underlining and bold"),
        )
        .arg(
            Arg::with_name(options::LINES)
                .short("n")
                .long(options::LINES)
                .value_name("number")
                .takes_value(true)
                .help("The number of lines per screen full"),
        )
        .arg(
            Arg::with_name(options::NUMBER)
                .allow_hyphen_values(true)
                .long(options::NUMBER)
                .required(false)
                .takes_value(true)
                .help("Same as --lines"),
        )
        .arg(
            Arg::with_name(options::FROM_LINE)
                .short("F")
                .allow_hyphen_values(true)
                .required(false)
                .takes_value(true)
                .value_name("number")
                .help("Display file beginning from line number"),
        )
        .arg(
            Arg::with_name(options::PATTERN)
                .short("P")
                .allow_hyphen_values(true)
                .required(false)
                .takes_value(true)
                .help("Display file beginning from pattern match"),
        )
        */
        .arg(
            Arg::with_name(options::FILES)
                .required(false)
                .multiple(true)
                .help("Path to the files to be read"),
        )
}
