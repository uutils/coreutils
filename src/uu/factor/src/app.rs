use clap::{crate_version, App, Arg};

pub mod options {
    pub const NUMBER: &str = "NUMBER";
}

const SUMMARY: &str = "Print the prime factors of the given NUMBER(s).
If none are specified, read from standard input.";

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .about(SUMMARY)
        .arg(Arg::with_name(options::NUMBER).multiple(true))
}
