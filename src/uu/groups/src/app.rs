use clap::{crate_version, App, Arg};

const ABOUT: &str = "display current group names";
pub const OPT_USER: &str = "user";

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .about(ABOUT)
        .arg(Arg::with_name(OPT_USER))
}
