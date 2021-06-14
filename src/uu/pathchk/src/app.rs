use clap::{crate_version, App, Arg};

const ABOUT: &str = "Check whether file names are valid or portable";

pub mod options {
    pub const POSIX: &str = "posix";
    pub const POSIX_SPECIAL: &str = "posix-special";
    pub const PORTABILITY: &str = "portability";
    pub const PATH: &str = "path";
}

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .about(ABOUT)
        .arg(
            Arg::with_name(options::POSIX)
                .short("p")
                .help("check for most POSIX systems"),
        )
        .arg(
            Arg::with_name(options::POSIX_SPECIAL)
                .short("P")
                .help(r#"check for empty names and leading "-""#),
        )
        .arg(
            Arg::with_name(options::PORTABILITY)
                .long(options::PORTABILITY)
                .help("check for all POSIX systems (equivalent to -p -P)"),
        )
        .arg(Arg::with_name(options::PATH).hidden(true).multiple(true))
}
