use clap::{crate_version, App, Arg};

static SYNTAX: &str = "[OPTION]... [FILE]...";
static SUMMARY: &str = "Writes each file (or standard input if no files are given)
 to standard output whilst breaking long lines";

pub mod options {
    pub const BYTES: &str = "bytes";
    pub const SPACES: &str = "spaces";
    pub const WIDTH: &str = "width";
    pub const FILE: &str = "file";
}

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .usage(SYNTAX)
        .about(SUMMARY)
        .arg(
            Arg::with_name(options::BYTES)
                .long(options::BYTES)
                .short("b")
                .help(
                    "count using bytes rather than columns (meaning control characters \
                 such as newline are not treated specially)",
                )
                .takes_value(false),
        )
        .arg(
            Arg::with_name(options::SPACES)
                .long(options::SPACES)
                .short("s")
                .help("break lines at word boundaries rather than a hard cut-off")
                .takes_value(false),
        )
        .arg(
            Arg::with_name(options::WIDTH)
                .long(options::WIDTH)
                .short("w")
                .help("set WIDTH as the maximum line width rather than 80")
                .value_name("WIDTH")
                .allow_hyphen_values(true)
                .takes_value(true),
        )
        .arg(Arg::with_name(options::FILE).hidden(true).multiple(true))
}
