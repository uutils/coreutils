use clap::{crate_version, App, Arg};

const ABOUT: &str = "Display file or file system status.";

pub mod options {
    pub const DEREFERENCE: &str = "dereference";
    pub const FILE_SYSTEM: &str = "file-system";
    pub const FORMAT: &str = "format";
    pub const PRINTF: &str = "printf";
    pub const TERSE: &str = "terse";
}

pub const ARG_FILES: &str = "files";

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .about(ABOUT)
        .arg(
            Arg::with_name(options::DEREFERENCE)
                .short("L")
                .long(options::DEREFERENCE)
                .help("follow links"),
        )
        .arg(
            Arg::with_name(options::FILE_SYSTEM)
                .short("f")
                .long(options::FILE_SYSTEM)
                .help("display file system status instead of file status"),
        )
        .arg(
            Arg::with_name(options::TERSE)
                .short("t")
                .long(options::TERSE)
                .help("print the information in terse form"),
        )
        .arg(
            Arg::with_name(options::FORMAT)
                .short("c")
                .long(options::FORMAT)
                .help(
                    "use the specified FORMAT instead of the default;
output a newline after each use of FORMAT",
                )
                .value_name("FORMAT"),
        )
        .arg(
            Arg::with_name(options::PRINTF)
                .long(options::PRINTF)
                .value_name("FORMAT")
                .help(
                    "like --format, but interpret backslash escapes,
        and do not output a mandatory trailing newline;
        if you want a newline, include \n in FORMAT",
                ),
        )
        .arg(
            Arg::with_name(ARG_FILES)
                .multiple(true)
                .takes_value(true)
                .min_values(1),
        )
}
