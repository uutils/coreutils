use clap::{crate_version, App, Arg};

const ABOUT: &str = "strip last component from file name";

pub mod options {
    pub const ZERO: &str = "zero";
    pub const DIR: &str = "dir";
}

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .about(ABOUT)
        .version(crate_version!())
        .arg(
            Arg::with_name(options::ZERO)
                .long(options::ZERO)
                .short("z")
                .help("separate output with NUL rather than newline"),
        )
        .arg(Arg::with_name(options::DIR).hidden(true).multiple(true))
}
