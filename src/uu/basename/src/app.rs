use clap::{crate_version, App, Arg};

const SUMMARY: &str = "Print NAME with any leading directory components removed
If specified, also remove a trailing SUFFIX";

pub mod options {
    pub const MULTIPLE: &str = "multiple";
    pub const NAME: &str = "name";
    pub const SUFFIX: &str = "suffix";
    pub const ZERO: &str = "zero";
}

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .about(SUMMARY)
        .arg(
            Arg::with_name(options::MULTIPLE)
                .short("a")
                .long(options::MULTIPLE)
                .help("support multiple arguments and treat each as a NAME"),
        )
        .arg(Arg::with_name(options::NAME).multiple(true).hidden(true))
        .arg(
            Arg::with_name(options::SUFFIX)
                .short("s")
                .long(options::SUFFIX)
                .value_name("SUFFIX")
                .help("remove a trailing SUFFIX; implies -a"),
        )
        .arg(
            Arg::with_name(options::ZERO)
                .short("z")
                .long(options::ZERO)
                .help("end each output line with NUL, not newline"),
        )
}
