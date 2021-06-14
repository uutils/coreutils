use clap::{crate_version, App, Arg};

const ABOUT: &str = "Display the values of the specified environment VARIABLE(s), or (with no VARIABLE) display name and value pairs for them all.";

pub const OPT_NULL: &str = "null";

pub const ARG_VARIABLES: &str = "variables";

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .about(ABOUT)
        .arg(
            Arg::with_name(OPT_NULL)
                .short("0")
                .long(OPT_NULL)
                .help("end each output line with 0 byte rather than newline"),
        )
        .arg(
            Arg::with_name(ARG_VARIABLES)
                .multiple(true)
                .takes_value(true)
                .min_values(1),
        )
}
