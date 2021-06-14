use clap::App;

pub fn get_app(app_name: &str) -> App {
    // TOOD: migrate to clap.
    App::new(app_name)
}
