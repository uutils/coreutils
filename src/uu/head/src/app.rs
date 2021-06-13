use clap::{crate_version, App, Arg};

const ABOUT: &str = "\
Print the first 10 lines of each FILE to standard output.\n\
With more than one FILE, precede each with a header giving the file name.\n\
\n\
With no FILE, or when FILE is -, read standard input.\n\
\n\
Mandatory arguments to long flags are mandatory for short flags too.\
";
const USAGE: &str = "head [FLAG]... [FILE]...";

pub mod options {
    pub const BYTES_NAME: &str = "BYTES";
    pub const LINES_NAME: &str = "LINES";
    pub const QUIET_NAME: &str = "QUIET";
    pub const VERBOSE_NAME: &str = "VERBOSE";
    pub const ZERO_NAME: &str = "ZERO";
    pub const FILES_NAME: &str = "FILE";
}

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .about(ABOUT)
        .usage(USAGE)
        .arg(
            Arg::with_name(options::BYTES_NAME)
                .short("c")
                .long("bytes")
                .value_name("[-]NUM")
                .takes_value(true)
                .help(
                    "\
                     print the first NUM bytes of each file;\n\
                     with the leading '-', print all but the last\n\
                     NUM bytes of each file\
                     ",
                )
                .overrides_with_all(&[options::BYTES_NAME, options::LINES_NAME])
                .allow_hyphen_values(true),
        )
        .arg(
            Arg::with_name(options::LINES_NAME)
                .short("n")
                .long("lines")
                .value_name("[-]NUM")
                .takes_value(true)
                .help(
                    "\
                     print the first NUM lines instead of the first 10;\n\
                     with the leading '-', print all but the last\n\
                     NUM lines of each file\
                     ",
                )
                .overrides_with_all(&[options::LINES_NAME, options::BYTES_NAME])
                .allow_hyphen_values(true),
        )
        .arg(
            Arg::with_name(options::QUIET_NAME)
                .short("q")
                .long("quiet")
                .visible_alias("silent")
                .help("never print headers giving file names")
                .overrides_with_all(&[options::VERBOSE_NAME, options::QUIET_NAME]),
        )
        .arg(
            Arg::with_name(options::VERBOSE_NAME)
                .short("v")
                .long("verbose")
                .help("always print headers giving file names")
                .overrides_with_all(&[options::QUIET_NAME, options::VERBOSE_NAME]),
        )
        .arg(
            Arg::with_name(options::ZERO_NAME)
                .short("z")
                .long("zero-terminated")
                .help("line delimiter is NUL, not newline")
                .overrides_with(options::ZERO_NAME),
        )
        .arg(Arg::with_name(options::FILES_NAME).multiple(true))
}
