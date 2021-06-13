use clap::{crate_version, App, Arg};

const ABOUT: &str = "Display user and group information for the specified USER,\n or (when USER omitted) for the current user.";

pub const OPT_AUDIT: &str = "audit";
pub const OPT_EFFECTIVE_USER: &str = "effective-user";
pub const OPT_GROUP: &str = "group";
pub const OPT_GROUPS: &str = "groups";
pub const OPT_HUMAN_READABLE: &str = "human-readable";
pub const OPT_NAME: &str = "name";
pub const OPT_PASSWORD: &str = "password";
pub const OPT_REAL_ID: &str = "real";

pub const ARG_USERS: &str = "users";

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .about(ABOUT)
        .arg(
            Arg::with_name(OPT_AUDIT)
                .short("A")
                .help("Display the process audit (not available on Linux)"),
        )
        .arg(
            Arg::with_name(OPT_EFFECTIVE_USER)
                .short("u")
                .long("user")
                .help("Display the effective user ID as a number"),
        )
        .arg(
            Arg::with_name(OPT_GROUP)
                .short("g")
                .long(OPT_GROUP)
                .help("Display the effective group ID as a number"),
        )
        .arg(
            Arg::with_name(OPT_GROUPS)
                .short("G")
                .long(OPT_GROUPS)
                .help("Display the different group IDs"),
        )
        .arg(
            Arg::with_name(OPT_HUMAN_READABLE)
                .short("p")
                .help("Make the output human-readable"),
        )
        .arg(
            Arg::with_name(OPT_NAME)
                .short("n")
                .help("Display the name of the user or group ID for the -G, -g and -u options"),
        )
        .arg(
            Arg::with_name(OPT_PASSWORD)
                .short("P")
                .help("Display the id as a password file entry"),
        )
        .arg(
            Arg::with_name(OPT_REAL_ID)
                .short("r")
                .long(OPT_REAL_ID)
                .help(
                "Display the real ID for the -G, -g and -u options instead of the effective ID.",
            ),
        )
        .arg(Arg::with_name(ARG_USERS).multiple(true).takes_value(true))
}
