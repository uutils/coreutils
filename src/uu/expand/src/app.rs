use clap::{crate_version, App, Arg};

const ABOUT: &str = "Convert tabs in each FILE to spaces, writing to standard output.
 With no FILE, or when FILE is -, read standard input.";

const LONG_HELP: &str = "";

pub mod options {
    pub static TABS: &str = "tabs";
    pub static INITIAL: &str = "initial";
    pub static NO_UTF8: &str = "no-utf8";
    pub static FILES: &str = "FILES";
}

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .about(ABOUT)
        .after_help(LONG_HELP)
        .arg(
            Arg::with_name(options::INITIAL)
                .long(options::INITIAL)
                .short("i")
                .help("do not convert tabs after non blanks"),
        )
        .arg(
            Arg::with_name(options::TABS)
                .long(options::TABS)
                .short("t")
                .value_name("N, LIST")
                .takes_value(true)
                .help("have tabs N characters apart, not 8 or use comma separated list of explicit tab positions"),
        )
        .arg(
            Arg::with_name(options::NO_UTF8)
                .long(options::NO_UTF8)
                .short("U")
                .help("interpret input file as 8-bit ASCII rather than UTF-8"),
        ).arg(
            Arg::with_name(options::FILES)
                .multiple(true)
                .hidden(true)
                .takes_value(true)
        )
}
