use clap::{crate_version, App};

pub fn get_app(app_name: &str) -> App {
    App::new(app_name).version(crate_version!())
}
