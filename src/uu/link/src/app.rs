use clap::{crate_version, App, Arg};

const ABOUT: &str = "Call the link function to create a link named FILE2 to an existing FILE1.";

pub mod options {
    pub const FILES: &str = "FILES";
}

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .about(ABOUT)
        .arg(
            Arg::with_name(options::FILES)
                .hidden(true)
                .required(true)
                .min_values(2)
                .max_values(2)
                .takes_value(true),
        )
}
