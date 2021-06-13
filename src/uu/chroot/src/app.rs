// spell-checker:ignore (ToDO) NEWROOT Userspec pstatus

use clap::{crate_version, App, Arg};

mod options {
    pub const NEWROOT: &str = "newroot";
    pub const USER: &str = "user";
    pub const GROUP: &str = "group";
    pub const GROUPS: &str = "groups";
    pub const USERSPEC: &str = "userspec";
    pub const COMMAND: &str = "command";
}

const ABOUT: &str = "Run COMMAND with root directory set to NEWROOT.";
const SYNTAX: &str = "[OPTION]... NEWROOT [COMMAND [ARG]...]";

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .about(ABOUT)
        .usage(SYNTAX)
        .arg(
            Arg::with_name(options::NEWROOT)
                .hidden(true)
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name(options::USER)
                .short("u")
                .long(options::USER)
                .help("User (ID or name) to switch before running the program")
                .value_name("USER"),
        )
        .arg(
            Arg::with_name(options::GROUP)
                .short("g")
                .long(options::GROUP)
                .help("Group (ID or name) to switch to")
                .value_name("GROUP"),
        )
        .arg(
            Arg::with_name(options::GROUPS)
                .short("G")
                .long(options::GROUPS)
                .help("Comma-separated list of groups to switch to")
                .value_name("GROUP1,GROUP2..."),
        )
        .arg(
            Arg::with_name(options::USERSPEC)
                .long(options::USERSPEC)
                .help(
                    "Colon-separated user and group to switch to. \
                     Same as -u USER -g GROUP. \
                     Userspec has higher preference than -u and/or -g",
                )
                .value_name("USER:GROUP"),
        )
        .arg(
            Arg::with_name(options::COMMAND)
                .hidden(true)
                .multiple(true)
                .index(2),
        )
}
