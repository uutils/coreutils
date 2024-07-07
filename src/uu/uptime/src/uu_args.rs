// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{builder::ValueParser, crate_version, Arg, ArgAction, Command, ValueHint};
use uucore::{format_usage, help_about, help_usage};

const ABOUT: &str = help_about!("uptime.md");
const USAGE: &str = help_usage!("uptime.md");

pub mod options {
    pub static SINCE: &str = "since";
    pub static PATH: &str = "path";
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::SINCE)
                .short('s')
                .long(options::SINCE)
                .help("system up since")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::PATH)
                .help("file to search boot time from")
                .action(ArgAction::Append)
                .value_parser(ValueParser::os_string())
                .value_hint(ValueHint::AnyPath),
        )
}
