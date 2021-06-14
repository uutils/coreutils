use clap::{crate_version, App};

const ABOUT: &str = "Display machine architecture";
const SUMMARY: &str = "Determine architecture name for current machine.";

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .about(ABOUT)
        .after_help(SUMMARY)
}
