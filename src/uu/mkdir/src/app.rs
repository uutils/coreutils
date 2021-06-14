use clap::{crate_version, App, Arg};

const ABOUT: &str = "Create the given DIRECTORY(ies) if they do not exist";
pub const OPT_MODE: &str = "mode";
pub const OPT_PARENTS: &str = "parents";
pub const OPT_VERBOSE: &str = "verbose";

pub const ARG_DIRS: &str = "dirs";

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .about(ABOUT)
        .arg(
            Arg::with_name(OPT_MODE)
                .short("m")
                .long(OPT_MODE)
                .help("set file mode")
                .default_value("755"),
        )
        .arg(
            Arg::with_name(OPT_PARENTS)
                .short("p")
                .long(OPT_PARENTS)
                .alias("parent")
                .help("make parent directories as needed"),
        )
        .arg(
            Arg::with_name(OPT_VERBOSE)
                .short("v")
                .long(OPT_VERBOSE)
                .help("print a message for each printed directory"),
        )
        .arg(
            Arg::with_name(ARG_DIRS)
                .multiple(true)
                .takes_value(true)
                .min_values(1),
        )
}
