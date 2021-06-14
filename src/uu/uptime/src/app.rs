use clap::{crate_version, App, Arg};

const ABOUT: &str = "Display the current time, the length of time the system has been up,\n\
                      the number of users on the system, and the average number of jobs\n\
                      in the run queue over the last 1, 5 and 15 minutes.";
pub mod options {
    pub const SINCE: &str = "since";
}

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .about(ABOUT)
        .arg(
            Arg::with_name(options::SINCE)
                .short("s")
                .long(options::SINCE)
                .help("system up since"),
        )
}
