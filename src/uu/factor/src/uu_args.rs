// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{crate_version, Arg, ArgAction, Command};
use uucore::{format_usage, help_about, help_usage};

const ABOUT: &str = help_about!("factor.md");
const USAGE: &str = help_usage!("factor.md");

pub mod options {
    pub static EXPONENTS: &str = "exponents";
    pub static HELP: &str = "help";
    pub static NUMBER: &str = "NUMBER";
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .disable_help_flag(true)
        .args_override_self(true)
        .arg(Arg::new(options::NUMBER).action(ArgAction::Append))
        .arg(
            Arg::new(options::EXPONENTS)
                .short('h')
                .long(options::EXPONENTS)
                .help("Print factors in the form p^e")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::HELP)
                .long(options::HELP)
                .help("Print help information.")
                .action(ArgAction::Help),
        )
}
