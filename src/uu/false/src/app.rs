use clap::{App, AppSettings};

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .setting(AppSettings::DisableHelpFlags)
        .setting(AppSettings::DisableVersion)
}
