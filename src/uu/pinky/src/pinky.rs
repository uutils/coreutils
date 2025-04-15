// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) BUFSIZE gecos fullname, mesg iobuf

use clap::{Arg, ArgAction, Command};
use uucore::{format_usage, help_about, help_usage};

mod platform;

#[cfg(target_env = "musl")]
const ABOUT: &str = concat!(
    help_about!("pinky.md"),
    "\n\nWarning: When built with musl libc, the `pinky` utility may show incomplete \n",
    "or missing user information due to musl's stub implementation of `utmpx` \n",
    "functions. This limitation affects the ability to retrieve accurate details \n",
    "about logged-in users."
);

#[cfg(not(target_env = "musl"))]
const ABOUT: &str = help_about!("pinky.md");

const USAGE: &str = help_usage!("pinky.md");

mod options {
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
    pub const HELP: &str = "help";
}

#[uucore::main]
use platform::uumain;

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .disable_help_flag(true)
        .arg(
            Arg::new(options::LONG_FORMAT)
                .short('l')
                .requires(options::USER)
                .help("produce long format output for the specified USERs")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OMIT_HOME_DIR)
                .short('b')
                .help("omit the user's home directory and shell in long format")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OMIT_PROJECT_FILE)
                .short('h')
                .help("omit the user's project file in long format")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OMIT_PLAN_FILE)
                .short('p')
                .help("omit the user's plan file in long format")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SHORT_FORMAT)
                .short('s')
                .help("do short format output, this is the default")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OMIT_HEADINGS)
                .short('f')
                .help("omit the line of column headings in short format")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OMIT_NAME)
                .short('w')
                .help("omit the user's full name in short format")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OMIT_NAME_HOST)
                .short('i')
                .help("omit the user's full name and remote host in short format")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OMIT_NAME_HOST_TIME)
                .short('q')
                .help("omit the user's full name, remote host and idle time in short format")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::USER)
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::Username),
        )
        .arg(
            // Redefine the help argument to not include the short flag
            // since that conflicts with omit_project_file.
            Arg::new(options::HELP)
                .long(options::HELP)
                .help("Print help information")
                .action(ArgAction::Help),
        )
}

pub trait Capitalize {
    fn capitalize(&self) -> String;
}

impl Capitalize for str {
    fn capitalize(&self) -> String {
        self.char_indices()
            .fold(String::with_capacity(self.len()), |mut acc, x| {
                if x.0 == 0 {
                    acc.push(x.1.to_ascii_uppercase());
                } else {
                    acc.push(x.1);
                }
                acc
            })
    }
}
