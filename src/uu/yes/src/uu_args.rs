// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use clap::{builder::ValueParser, crate_version, Arg, ArgAction, Command};
use uucore::{format_usage, help_about, help_usage};

const ABOUT: &str = help_about!("yes.md");
const USAGE: &str = help_usage!("yes.md");

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .arg(
            Arg::new("STRING")
                .value_parser(ValueParser::os_string())
                .action(ArgAction::Append),
        )
        .infer_long_args(true)
}
