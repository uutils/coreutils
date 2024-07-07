// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{crate_version, Arg, ArgAction, Command};

use uucore::{format_usage, help_about, help_section, help_usage};

const ABOUT: &str = help_about!("dirname.md");
const USAGE: &str = help_usage!("dirname.md");
const AFTER_HELP: &str = help_section!("after help", "dirname.md");

pub mod options {
    pub const ZERO: &str = "zero";
    pub const DIR: &str = "dir";
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .about(ABOUT)
        .version(crate_version!())
        .override_usage(format_usage(USAGE))
        .args_override_self(true)
        .infer_long_args(true)
        .after_help(AFTER_HELP)
        .arg(
            Arg::new(options::ZERO)
                .long(options::ZERO)
                .short('z')
                .help("separate output with NUL rather than newline")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::DIR)
                .hide(true)
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::AnyPath),
        )
}
