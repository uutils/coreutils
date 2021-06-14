use clap::{crate_version, App, Arg};

const ABOUT: &str = "Print the user names of users currently logged in to the current host";

pub const ARG_FILES: &str = "files";

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .about(ABOUT)
        .arg(Arg::with_name(ARG_FILES).takes_value(true).max_values(1))
}
