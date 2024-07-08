// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{crate_version, Arg, ArgAction, Command};
use uucore::{format_usage, help_about, help_usage};
const ABOUT: &str = help_about!("nice.md");
const USAGE: &str = help_usage!("nice.md");

pub mod options {
    pub static ADJUSTMENT: &str = "adjustment";
    pub static COMMAND: &str = "COMMAND";
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .trailing_var_arg(true)
        .infer_long_args(true)
        .version(crate_version!())
        .arg(
            Arg::new(options::ADJUSTMENT)
                .short('n')
                .long(options::ADJUSTMENT)
                .help("add N to the niceness (default is 10)")
                .action(ArgAction::Set)
                .overrides_with(options::ADJUSTMENT)
                .allow_hyphen_values(true),
        )
        .arg(
            Arg::new(options::COMMAND)
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::CommandName),
        )
}
