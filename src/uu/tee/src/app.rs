use clap::{crate_version, App, Arg};

const ABOUT: &str = "Copy standard input to each FILE, and also to standard output.";

pub mod options {
    pub const APPEND: &str = "append";
    pub const IGNORE_INTERRUPTS: &str = "ignore-interrupts";
    pub const FILE: &str = "file";
}

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .about(ABOUT)
        .after_help("If a FILE is -, it refers to a file named - .")
        .arg(
            Arg::with_name(options::APPEND)
                .long(options::APPEND)
                .short("a")
                .help("append to the given FILEs, do not overwrite"),
        )
        .arg(
            Arg::with_name(options::IGNORE_INTERRUPTS)
                .long(options::IGNORE_INTERRUPTS)
                .short("i")
                .help("ignore interrupt signals (ignored on non-Unix platforms)"),
        )
        .arg(Arg::with_name(options::FILE).multiple(true))
}
