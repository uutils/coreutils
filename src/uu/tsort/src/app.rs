use clap::{crate_version, App, Arg};

const SUMMARY: &str = "Topological sort the strings in FILE.
Strings are defined as any sequence of tokens separated by whitespace (tab, space, or newline).
If FILE is not passed in, stdin is used instead.";
const USAGE: &str = "tsort [OPTIONS] FILE";

pub mod options {
    pub const FILE: &str = "file";
}

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .usage(USAGE)
        .about(SUMMARY)
        .arg(
            Arg::with_name(options::FILE)
                .default_value("-")
                .hidden(true),
        )
}
