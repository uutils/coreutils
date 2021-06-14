use clap::{crate_version, App, Arg};

const NAME: &str = "unexpand";
const USAGE: &str = "unexpand [OPTION]... [FILE]...";
const SUMMARY: &str = "Convert blanks in each FILE to tabs, writing to standard output.\n
                 With no FILE, or when FILE is -, read standard input.";

pub mod options {
    pub const FILE: &str = "file";
    pub const ALL: &str = "all";
    pub const FIRST_ONLY: &str = "first-only";
    pub const TABS: &str = "tabs";
    pub const NO_UTF8: &str = "no-utf8";
}

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .name(NAME)
        .version(crate_version!())
        .usage(USAGE)
        .about(SUMMARY)
        .arg(Arg::with_name(options::FILE).hidden(true).multiple(true))
        .arg(
            Arg::with_name(options::ALL)
                .short("a")
                .long(options::ALL)
                .help("convert all blanks, instead of just initial blanks")
                .takes_value(false),
        )
        .arg(
            Arg::with_name(options::FIRST_ONLY)
                .long(options::FIRST_ONLY)
                .help("convert only leading sequences of blanks (overrides -a)")
                .takes_value(false),
        )
        .arg(
            Arg::with_name(options::TABS)
                .short("t")
                .long(options::TABS)
                .long_help("use comma separated LIST of tab positions or have tabs N characters apart instead of 8 (enables -a)")
                .takes_value(true)
        )
        .arg(
            Arg::with_name(options::NO_UTF8)
                .short("U")
                .long(options::NO_UTF8)
                .takes_value(false)
                .help("interpret input file as 8-bit ASCII rather than UTF-8"))
}
