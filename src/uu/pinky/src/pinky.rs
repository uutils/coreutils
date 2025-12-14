// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) BUFSIZE gecos fullname, mesg iobuf

use clap::{Arg, ArgAction, Command};
use uucore::format_usage;
use uucore::translate;

mod platform;

mod options {
    pub const LONG_FORMAT: &str = "long_format";
    pub const LOOKUP: &str = "lookup";
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
    #[cfg(not(target_env = "musl"))]
    let about = translate!("pinky-about");
    #[cfg(target_env = "musl")]
    let about = translate!("pinky-about") + &translate!("pinky-about-musl-warning");

    let cmd = Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .about(about)
        .override_usage(format_usage(&translate!("pinky-usage")))
        .infer_long_args(true);
    uucore::clap_localization::configure_localized_command(cmd)
        .disable_help_flag(true)
        .arg(
            Arg::new(options::LONG_FORMAT)
                .short('l')
                .requires(options::USER)
                .help(translate!("pinky-help-long-format"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OMIT_HOME_DIR)
                .short('b')
                .help(translate!("pinky-help-omit-home-dir"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OMIT_PROJECT_FILE)
                .short('h')
                .help(translate!("pinky-help-omit-project-file"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OMIT_PLAN_FILE)
                .short('p')
                .help(translate!("pinky-help-omit-plan-file"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SHORT_FORMAT)
                .short('s')
                .help(translate!("pinky-help-short-format"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OMIT_HEADINGS)
                .short('f')
                .help(translate!("pinky-help-omit-headings"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OMIT_NAME)
                .short('w')
                .help(translate!("pinky-help-omit-name"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OMIT_NAME_HOST)
                .short('i')
                .help(translate!("pinky-help-omit-name-host"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OMIT_NAME_HOST_TIME)
                .short('q')
                .help(translate!("pinky-help-omit-name-host-time"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::USER)
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::Username),
        )
        .arg(
            Arg::new(options::LOOKUP)
                .long(options::LOOKUP)
                .help(translate!("pinky-help-lookup"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            // Redefine the help argument to not include the short flag
            // since that conflicts with omit_project_file.
            Arg::new(options::HELP)
                .long(options::HELP)
                .help(translate!("pinky-help-help"))
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_capitalize() {
        assert_eq!("Zbnmasd", "zbnmasd".capitalize()); // spell-checker:disable-line
        assert_eq!("Abnmasd", "Abnmasd".capitalize()); // spell-checker:disable-line
        assert_eq!("1masd", "1masd".capitalize()); // spell-checker:disable-line
        assert_eq!("", "".capitalize());
    }
}
