use clap::{crate_version, App, Arg};

pub static ABOUT: &str = "Print the file name of the terminal connected to standard input.";

pub mod options {
    pub const SILENT: &str = "silent";
}

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .about(ABOUT)
        .arg(
            Arg::with_name(options::SILENT)
                .long(options::SILENT)
                .visible_alias("quiet")
                .short("s")
                .help("print nothing, only return an exit status")
                .required(false),
        )
}
