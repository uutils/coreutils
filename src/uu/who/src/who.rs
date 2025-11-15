// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) runlevel mesg

use clap::{Arg, ArgAction, Command};
use uucore::format_usage;
use uucore::translate;

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

fn get_runlevel_help() -> String {
    #[cfg(target_os = "linux")]
    return translate!("who-help-runlevel");
    #[cfg(not(target_os = "linux"))]
    return translate!("who-help-runlevel-non-linux");
}

#[uucore::main]
use platform::uumain;

pub fn uu_app() -> Command {
    #[cfg(not(target_env = "musl"))]
    let about = translate!("who-about");
    #[cfg(target_env = "musl")]
    let about = translate!("who-about") + &translate!("who-about-musl-warning");

    let cmd = Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .about(about)
        .override_usage(format_usage(&translate!("who-usage")))
        .infer_long_args(true);
    uucore::clap_localization::configure_localized_command(cmd)
        .arg(
            Arg::new(options::ALL)
                .long(options::ALL)
                .short('a')
                .help(translate!("who-help-all"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::BOOT)
                .long(options::BOOT)
                .short('b')
                .help(translate!("who-help-boot"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::DEAD)
                .long(options::DEAD)
                .short('d')
                .help(translate!("who-help-dead"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::HEADING)
                .long(options::HEADING)
                .short('H')
                .help(translate!("who-help-heading"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::LOGIN)
                .long(options::LOGIN)
                .short('l')
                .help(translate!("who-help-login"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::LOOKUP)
                .long(options::LOOKUP)
                .help(translate!("who-help-lookup"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::ONLY_HOSTNAME_USER)
                .short('m')
                .help(translate!("who-help-only-hostname-user"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::PROCESS)
                .long(options::PROCESS)
                .short('p')
                .help(translate!("who-help-process"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::COUNT)
                .long(options::COUNT)
                .short('q')
                .help(translate!("who-help-count"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::RUNLEVEL)
                .long(options::RUNLEVEL)
                .short('r')
                .help(get_runlevel_help())
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SHORT)
                .long(options::SHORT)
                .short('s')
                .help(translate!("who-help-short"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::TIME)
                .long(options::TIME)
                .short('t')
                .help(translate!("who-help-time"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::USERS)
                .long(options::USERS)
                .short('u')
                .help(translate!("who-help-users"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::MESG)
                .long(options::MESG)
                .short('T')
                .visible_short_alias('w')
                .visible_aliases(["message", "writable"])
                .help(translate!("who-help-mesg"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::FILE)
                .num_args(1..=2)
                .value_hint(clap::ValueHint::FilePath),
        )
}
