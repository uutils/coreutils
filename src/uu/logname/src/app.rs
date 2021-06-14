use clap::{crate_version, App};

const SUMMARY: &str = "Print user's login name";

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .usage(app_name)
        .version(crate_version!())
        .about(SUMMARY)
}
