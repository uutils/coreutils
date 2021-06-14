use clap::{crate_version, App, Arg};

const ABOUT: &str = "pinky - lightweight finger";

pub mod options {
    pub const LONG_FORMAT: &str = "long_format";
    pub const OMIT_HOME_DIR: &str = "omit_home_dir";
    pub const OMIT_PROJECT_FILE: &str = "omit_project_file";
    pub const OMIT_PLAN_FILE: &str = "omit_plan_file";
    pub const SHORT_FORMAT: &str = "short_format";
    pub const OMIT_HEADINGS: &str = "omit_headings";
    pub const OMIT_NAME: &str = "omit_name";
    pub const OMIT_NAME_HOST: &str = "omit_name_host";
    pub const OMIT_NAME_HOST_TIME: &str = "omit_name_host_time";
    pub const USER: &str = "user";
}

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .about(ABOUT)
        .arg(
            Arg::with_name(options::LONG_FORMAT)
                .short("l")
                .requires(options::USER)
                .help("produce long format output for the specified USERs"),
        )
        .arg(
            Arg::with_name(options::OMIT_HOME_DIR)
                .short("b")
                .help("omit the user's home directory and shell in long format"),
        )
        .arg(
            Arg::with_name(options::OMIT_PROJECT_FILE)
                .short("h")
                .help("omit the user's project file in long format"),
        )
        .arg(
            Arg::with_name(options::OMIT_PLAN_FILE)
                .short("p")
                .help("omit the user's plan file in long format"),
        )
        .arg(
            Arg::with_name(options::SHORT_FORMAT)
                .short("s")
                .help("do short format output, this is the default"),
        )
        .arg(
            Arg::with_name(options::OMIT_HEADINGS)
                .short("f")
                .help("omit the line of column headings in short format"),
        )
        .arg(
            Arg::with_name(options::OMIT_NAME)
                .short("w")
                .help("omit the user's full name in short format"),
        )
        .arg(
            Arg::with_name(options::OMIT_NAME_HOST)
                .short("i")
                .help("omit the user's full name and remote host in short format"),
        )
        .arg(
            Arg::with_name(options::OMIT_NAME_HOST_TIME)
                .short("q")
                .help("omit the user's full name, remote host and idle time in short format"),
        )
        .arg(
            Arg::with_name(options::USER)
                .takes_value(true)
                .multiple(true),
        )
}
