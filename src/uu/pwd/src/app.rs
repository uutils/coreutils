use clap::{crate_version, App, Arg};

const ABOUT: &str = "Display the full filename of the current working directory.";
pub const OPT_LOGICAL: &str = "logical";
pub const OPT_PHYSICAL: &str = "physical";

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .about(ABOUT)
        .arg(
            Arg::with_name(OPT_LOGICAL)
                .short("L")
                .long(OPT_LOGICAL)
                .help("use PWD from environment, even if it contains symlinks"),
        )
        .arg(
            Arg::with_name(OPT_PHYSICAL)
                .short("P")
                .long(OPT_PHYSICAL)
                .help("avoid all symlinks"),
        )
}
