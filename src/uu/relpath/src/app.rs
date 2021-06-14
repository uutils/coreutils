use clap::{crate_version, App, Arg};

const ABOUT: &str = "Convert TO destination to the relative path from the FROM dir.
If FROM path is omitted, current working dir will be used.";

pub mod options {
    pub const DIR: &str = "DIR";
    pub const TO: &str = "TO";
    pub const FROM: &str = "FROM";
}

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .about(ABOUT)
        .arg(
            Arg::with_name(options::DIR)
                .short("d")
                .takes_value(true)
                .help("If any of FROM and TO is not subpath of DIR, output absolute path instead of relative"),
        )
        .arg(
            Arg::with_name(options::TO)
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name(options::FROM)
                .takes_value(true),
        )
}
