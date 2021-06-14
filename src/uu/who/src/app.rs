// spell-checker:ignore (ToDO) hostnames runlevel mesg

use clap::{crate_version, App, Arg};

const ABOUT: &str = "Print information about users who are currently logged in.";

#[cfg(any(target_os = "linux"))]
const RUNLEVEL_HELP: &str = "print current runlevel";
#[cfg(not(target_os = "linux"))]
const RUNLEVEL_HELP: &str = "print current runlevel (This is meaningless on non Linux)";

pub mod options {
    pub const ALL: &str = "all";
    pub const BOOT: &str = "boot";
    pub const DEAD: &str = "dead";
    pub const HEADING: &str = "heading";
    pub const LOGIN: &str = "login";
    pub const LOOKUP: &str = "lookup";
    pub const ONLY_HOSTNAME_USER: &str = "only_hostname_user";
    pub const PROCESS: &str = "process";
    pub const COUNT: &str = "count";
    pub const RUNLEVEL: &str = "runlevel";
    pub const SHORT: &str = "short";
    pub const TIME: &str = "time";
    pub const USERS: &str = "users";
    pub const MESG: &str = "mesg"; // aliases: --message, --writable
    pub const FILE: &str = "FILE"; // if length=1: FILE, if length=2: ARG1 ARG2
}

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .about(ABOUT)
        .arg(
            Arg::with_name(options::ALL)
                .long(options::ALL)
                .short("a")
                .help("same as -b -d --login -p -r -t -T -u"),
        )
        .arg(
            Arg::with_name(options::BOOT)
                .long(options::BOOT)
                .short("b")
                .help("time of last system boot"),
        )
        .arg(
            Arg::with_name(options::DEAD)
                .long(options::DEAD)
                .short("d")
                .help("print dead processes"),
        )
        .arg(
            Arg::with_name(options::HEADING)
                .long(options::HEADING)
                .short("H")
                .help("print line of column headings"),
        )
        .arg(
            Arg::with_name(options::LOGIN)
                .long(options::LOGIN)
                .short("l")
                .help("print system login processes"),
        )
        .arg(
            Arg::with_name(options::LOOKUP)
                .long(options::LOOKUP)
                .help("attempt to canonicalize hostnames via DNS"),
        )
        .arg(
            Arg::with_name(options::ONLY_HOSTNAME_USER)
                .short("m")
                .help("only hostname and user associated with stdin"),
        )
        .arg(
            Arg::with_name(options::PROCESS)
                .long(options::PROCESS)
                .short("p")
                .help("print active processes spawned by init"),
        )
        .arg(
            Arg::with_name(options::COUNT)
                .long(options::COUNT)
                .short("q")
                .help("all login names and number of users logged on"),
        )
        .arg(
            Arg::with_name(options::RUNLEVEL)
                .long(options::RUNLEVEL)
                .short("r")
                .help(RUNLEVEL_HELP),
        )
        .arg(
            Arg::with_name(options::SHORT)
                .long(options::SHORT)
                .short("s")
                .help("print only name, line, and time (default)"),
        )
        .arg(
            Arg::with_name(options::TIME)
                .long(options::TIME)
                .short("t")
                .help("print last system clock change"),
        )
        .arg(
            Arg::with_name(options::USERS)
                .long(options::USERS)
                .short("u")
                .help("list users logged in"),
        )
        .arg(
            Arg::with_name(options::MESG)
                .long(options::MESG)
                .short("T")
                // .visible_short_alias('w')  // TODO: requires clap "3.0.0-beta.2"
                .visible_aliases(&["message", "writable"])
                .help("add user's message status as +, - or ?"),
        )
        .arg(
            Arg::with_name("w") // work around for `Arg::visible_short_alias`
                .short("w")
                .help("same as -T"),
        )
        .arg(
            Arg::with_name(options::FILE)
                .takes_value(true)
                .min_values(1)
                .max_values(2),
        )
}
