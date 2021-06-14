use clap::{crate_version, App, Arg};

const ABOUT: &str = "Unlink the file at [FILE].";
pub const OPT_PATH: &str = "FILE";

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .about(ABOUT)
        .arg(Arg::with_name(OPT_PATH).hidden(true).multiple(true))
}
