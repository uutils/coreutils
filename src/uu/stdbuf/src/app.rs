use clap::{crate_version, App, AppSettings, Arg};

const ABOUT: &str = "Run COMMAND, with modified buffering operations for its standard streams.\n\n\
     Mandatory arguments to long options are mandatory for short options too.";
const LONG_HELP: &str = "If MODE is 'L' the corresponding stream will be line buffered.\n\
     This option is invalid with standard input.\n\n\
     If MODE is '0' the corresponding stream will be unbuffered.\n\n\
     Otherwise MODE is a number which may be followed by one of the following:\n\n\
     KB 1000, K 1024, MB 1000*1000, M 1024*1024, and so on for G, T, P, E, Z, Y.\n\
     In this case the corresponding stream will be fully buffered with the buffer size set to \
     MODE bytes.\n\n\
     NOTE: If COMMAND adjusts the buffering of its standard streams ('tee' does for e.g.) then \
     that will override corresponding settings changed by 'stdbuf'.\n\
     Also some filters (like 'dd' and 'cat' etc.) don't use streams for I/O, \
     and are thus unaffected by 'stdbuf' settings.\n";

pub mod options {
    pub const INPUT: &str = "input";
    pub const INPUT_SHORT: &str = "i";
    pub const OUTPUT: &str = "output";
    pub const OUTPUT_SHORT: &str = "o";
    pub const ERROR: &str = "error";
    pub const ERROR_SHORT: &str = "e";
    pub const COMMAND: &str = "command";
}

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .about(ABOUT)
        .after_help(LONG_HELP)
        .setting(AppSettings::TrailingVarArg)
        .arg(
            Arg::with_name(options::INPUT)
                .long(options::INPUT)
                .short(options::INPUT_SHORT)
                .help("adjust standard input stream buffering")
                .value_name("MODE")
                .required_unless_one(&[options::OUTPUT, options::ERROR]),
        )
        .arg(
            Arg::with_name(options::OUTPUT)
                .long(options::OUTPUT)
                .short(options::OUTPUT_SHORT)
                .help("adjust standard output stream buffering")
                .value_name("MODE")
                .required_unless_one(&[options::INPUT, options::ERROR]),
        )
        .arg(
            Arg::with_name(options::ERROR)
                .long(options::ERROR)
                .short(options::ERROR_SHORT)
                .help("adjust standard error stream buffering")
                .value_name("MODE")
                .required_unless_one(&[options::INPUT, options::OUTPUT]),
        )
        .arg(
            Arg::with_name(options::COMMAND)
                .multiple(true)
                .takes_value(true)
                .hidden(true)
                .required(true),
        )
}
