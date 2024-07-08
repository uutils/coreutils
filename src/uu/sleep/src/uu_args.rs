// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{crate_version, Arg, ArgAction, Command};
use uucore::{format_usage, help_about, help_section, help_usage};

static ABOUT: &str = help_about!("sleep.md");
const USAGE: &str = help_usage!("sleep.md");
static AFTER_HELP: &str = help_section!("after help", "sleep.md");

pub mod options {
    pub const NUMBER: &str = "NUMBER";
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .after_help(AFTER_HELP)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::NUMBER)
                .help("pause for NUMBER seconds")
                .value_name(options::NUMBER)
                .action(ArgAction::Append),
        )
}
