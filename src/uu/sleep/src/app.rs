use clap::{crate_version, App, Arg};

static ABOUT: &str = "Pause for NUMBER seconds.";
static LONG_HELP: &str = "Pause for NUMBER seconds.  SUFFIX may be 's' for seconds (the default),
'm' for minutes, 'h' for hours or 'd' for days.  Unlike most implementations
that require NUMBER be an integer, here NUMBER may be an arbitrary floating
point number.  Given two or more arguments, pause for the amount of time
specified by the sum of their values.";

pub mod options {
    pub const NUMBER: &str = "NUMBER";
}

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .about(ABOUT)
        .after_help(LONG_HELP)
        .arg(
            Arg::with_name(options::NUMBER)
                .long(options::NUMBER)
                .help("pause for NUMBER seconds")
                .value_name(options::NUMBER)
                .index(1)
                .multiple(true)
                .required(true),
        )
}
