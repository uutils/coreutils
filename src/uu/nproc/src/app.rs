use clap::{crate_version, App, Arg};

pub const OPT_ALL: &str = "all";
pub const OPT_IGNORE: &str = "ignore";

const ABOUT: &str = "Print the number of cores available to the current process.";

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .about(ABOUT)
        .arg(
            Arg::with_name(OPT_ALL)
                .short("")
                .long(OPT_ALL)
                .help("print the number of cores available to the system"),
        )
        .arg(
            Arg::with_name(OPT_IGNORE)
                .short("")
                .long(OPT_IGNORE)
                .takes_value(true)
                .help("ignore up to N cores"),
        )
}
