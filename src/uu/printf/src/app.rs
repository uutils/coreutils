use clap::{App, Arg};

const VERSION: &str = "version";
const HELP: &str = "help";

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .arg(Arg::with_name(VERSION).long(VERSION))
        .arg(Arg::with_name(HELP).long(HELP))
}
