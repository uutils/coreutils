use clap::{crate_description, crate_version, App, Arg};

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .about(crate_description!())
        .version(crate_version!())
        .arg(Arg::with_name("STRING").index(1).multiple(true))
}
