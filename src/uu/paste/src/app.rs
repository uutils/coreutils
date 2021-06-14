use clap::{crate_version, App, Arg};

const ABOUT: &str = "Write lines consisting of the sequentially corresponding lines from each
FILE, separated by TABs, to standard output.";

pub mod options {
    pub const DELIMITER: &str = "delimiters";
    pub const SERIAL: &str = "serial";
    pub const FILE: &str = "file";
}

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .about(ABOUT)
        .arg(
            Arg::with_name(options::SERIAL)
                .long(options::SERIAL)
                .short("s")
                .help("paste one file at a time instead of in parallel"),
        )
        .arg(
            Arg::with_name(options::DELIMITER)
                .long(options::DELIMITER)
                .short("d")
                .help("reuse characters from LIST instead of TABs")
                .value_name("LIST")
                .default_value("\t")
                .hide_default_value(true),
        )
        .arg(
            Arg::with_name(options::FILE)
                .value_name("FILE")
                .multiple(true)
                .default_value("-"),
        )
}
