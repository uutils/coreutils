// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::ArgAction;
use clap::{crate_version, Arg, Command};
use uucore::{format_usage, help_about, help_usage};

const ABOUT: &str = help_about!("pwd.md");
const USAGE: &str = help_usage!("pwd.md");

pub mod options {
    pub const OPT_LOGICAL: &str = "logical";
    pub const OPT_PHYSICAL: &str = "physical";
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::OPT_LOGICAL)
                .short('L')
                .long(options::OPT_LOGICAL)
                .help("use PWD from environment, even if it contains symlinks")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_PHYSICAL)
                .short('P')
                .long(options::OPT_PHYSICAL)
                .overrides_with(options::OPT_LOGICAL)
                .help("avoid all symlinks")
                .action(ArgAction::SetTrue),
        )
}
