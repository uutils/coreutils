use clap::{crate_version, App, AppSettings, Arg};

pub mod options {
    pub static ADJUSTMENT: &str = "adjustment";
    pub static COMMAND: &str = "COMMAND";
}

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .setting(AppSettings::TrailingVarArg)
        .version(crate_version!())
        .arg(
            Arg::with_name(options::ADJUSTMENT)
                .short("n")
                .long(options::ADJUSTMENT)
                .help("add N to the niceness (default is 10)")
                .takes_value(true)
                .allow_hyphen_values(true),
        )
        .arg(Arg::with_name(options::COMMAND).multiple(true))
}
