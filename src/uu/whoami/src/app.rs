use clap::{crate_description, crate_version, App};

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .about(crate_description!())
        .version(crate_version!())
}
