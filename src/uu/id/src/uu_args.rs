// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use clap::{crate_version, Arg, ArgAction, Command};
use uucore::{format_usage, help_about, help_section, help_usage};

const ABOUT: &str = help_about!("id.md");
const USAGE: &str = help_usage!("id.md");
const AFTER_HELP: &str = help_section!("after help", "id.md");

pub mod options {
    pub const OPT_AUDIT: &str = "audit"; // GNU's id does not have this
    pub const OPT_CONTEXT: &str = "context";
    pub const OPT_EFFECTIVE_USER: &str = "user";
    pub const OPT_GROUP: &str = "group";
    pub const OPT_GROUPS: &str = "groups";
    pub const OPT_HUMAN_READABLE: &str = "human-readable"; // GNU's id does not have this
    pub const OPT_NAME: &str = "name";
    pub const OPT_PASSWORD: &str = "password"; // GNU's id does not have this
    pub const OPT_REAL_ID: &str = "real";
    pub const OPT_ZERO: &str = "zero"; // BSD's id does not have this
    pub const ARG_USERS: &str = "USER";
}

#[cfg(not(feature = "selinux"))]
static CONTEXT_HELP_TEXT: &str = "print only the security context of the process (not enabled)";
#[cfg(feature = "selinux")]
static CONTEXT_HELP_TEXT: &str = "print only the security context of the process";

#[allow(clippy::too_many_lines)]
pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .after_help(AFTER_HELP)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .args_override_self(true)
        .arg(
            Arg::new(options::OPT_AUDIT)
                .short('A')
                .conflicts_with_all([
                    options::OPT_GROUP,
                    options::OPT_EFFECTIVE_USER,
                    options::OPT_HUMAN_READABLE,
                    options::OPT_PASSWORD,
                    options::OPT_GROUPS,
                    options::OPT_ZERO,
                ])
                .help(
                    "Display the process audit user ID and other process audit properties,\n\
                      which requires privilege (not available on Linux).",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_EFFECTIVE_USER)
                .short('u')
                .long(options::OPT_EFFECTIVE_USER)
                .conflicts_with(options::OPT_GROUP)
                .help("Display only the effective user ID as a number.")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_GROUP)
                .short('g')
                .long(options::OPT_GROUP)
                .conflicts_with(options::OPT_EFFECTIVE_USER)
                .help("Display only the effective group ID as a number")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_GROUPS)
                .short('G')
                .long(options::OPT_GROUPS)
                .conflicts_with_all([
                    options::OPT_GROUP,
                    options::OPT_EFFECTIVE_USER,
                    options::OPT_CONTEXT,
                    options::OPT_HUMAN_READABLE,
                    options::OPT_PASSWORD,
                    options::OPT_AUDIT,
                ])
                .help(
                    "Display only the different group IDs as white-space separated numbers, \
                      in no particular order.",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_HUMAN_READABLE)
                .short('p')
                .help("Make the output human-readable. Each display is on a separate line.")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_NAME)
                .short('n')
                .long(options::OPT_NAME)
                .help(
                    "Display the name of the user or group ID for the -G, -g and -u options \
                      instead of the number.\nIf any of the ID numbers cannot be mapped into \
                      names, the number will be displayed as usual.",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_PASSWORD)
                .short('P')
                .help("Display the id as a password file entry.")
                .conflicts_with(options::OPT_HUMAN_READABLE)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_REAL_ID)
                .short('r')
                .long(options::OPT_REAL_ID)
                .help(
                    "Display the real ID for the -G, -g and -u options instead of \
                      the effective ID.",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_ZERO)
                .short('z')
                .long(options::OPT_ZERO)
                .help(
                    "delimit entries with NUL characters, not whitespace;\n\
                      not permitted in default format",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_CONTEXT)
                .short('Z')
                .long(options::OPT_CONTEXT)
                .conflicts_with_all([options::OPT_GROUP, options::OPT_EFFECTIVE_USER])
                .help(CONTEXT_HELP_TEXT)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::ARG_USERS)
                .action(ArgAction::Append)
                .value_name(options::ARG_USERS)
                .value_hint(clap::ValueHint::Username),
        )
}
