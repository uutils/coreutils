// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) ttyname hostnames runlevel mesg wtmp statted boottime deadprocs initspawn clockchange curr runlvline pidstr exitstr hoststr

use clap::{Arg, ArgAction, Command};
use uucore::{format_usage, help_about, help_usage};

mod platform;

mod options {
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

#[cfg(target_env = "musl")]
const ABOUT: &str = concat!(
    help_about!("who.md"),
    "\n\nNote: When built with musl libc, the `who` utility will not display any \n",
    "information about logged-in users. This is due to musl's stub implementation \n",
    "of `utmpx` functions, which prevents access to the necessary data."
);

#[cfg(not(target_env = "musl"))]
const ABOUT: &str = help_about!("who.md");

const USAGE: &str = help_usage!("who.md");

#[cfg(target_os = "linux")]
static RUNLEVEL_HELP: &str = "print current runlevel";
#[cfg(not(target_os = "linux"))]
static RUNLEVEL_HELP: &str = "print current runlevel (This is meaningless on non Linux)";

#[uucore::main]
use platform::uumain;

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::ALL)
                .long(options::ALL)
                .short('a')
                .help("same as -b -d --login -p -r -t -T -u")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::BOOT)
                .long(options::BOOT)
                .short('b')
                .help("time of last system boot")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::DEAD)
                .long(options::DEAD)
                .short('d')
                .help("print dead processes")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::HEADING)
                .long(options::HEADING)
                .short('H')
                .help("print line of column headings")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::LOGIN)
                .long(options::LOGIN)
                .short('l')
                .help("print system login processes")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::LOOKUP)
                .long(options::LOOKUP)
                .help("attempt to canonicalize hostnames via DNS")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::ONLY_HOSTNAME_USER)
                .short('m')
                .help("only hostname and user associated with stdin")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::PROCESS)
                .long(options::PROCESS)
                .short('p')
                .help("print active processes spawned by init")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::COUNT)
                .long(options::COUNT)
                .short('q')
                .help("all login names and number of users logged on")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::RUNLEVEL)
                .long(options::RUNLEVEL)
                .short('r')
                .help(RUNLEVEL_HELP)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SHORT)
                .long(options::SHORT)
                .short('s')
                .help("print only name, line, and time (default)")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::TIME)
                .long(options::TIME)
                .short('t')
                .help("print last system clock change")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::USERS)
                .long(options::USERS)
                .short('u')
                .help("list users logged in")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::MESG)
                .long(options::MESG)
                .short('T')
                .visible_short_alias('w')
                .visible_aliases(["message", "writable"])
                .help("add user's message status as +, - or ?")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::FILE)
                .num_args(1..=2)
                .value_hint(clap::ValueHint::FilePath),
        )
}
