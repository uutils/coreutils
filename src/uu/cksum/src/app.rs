use clap::{crate_version, App, Arg};

const SYNTAX: &str = "[OPTIONS] [FILE]...";
const SUMMARY: &str = "Print CRC and size for each file";

pub mod options {
    pub static FILE: &str = "file";
}

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .about(SUMMARY)
        .usage(SYNTAX)
        .arg(Arg::with_name(options::FILE).hidden(true).multiple(true))
}
