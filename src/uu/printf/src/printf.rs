// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
#![allow(dead_code)]
// spell-checker:ignore (change!) each's
// spell-checker:ignore (ToDO) LONGHELP FORMATSTRING templating parameterizing formatstr

use clap::{crate_version, Arg, ArgAction, Command};
use uucore::error::{UResult, UUsageError};
use uucore::memo::printf;
use uucore::{format_usage, help_about, help_section, help_usage};

const VERSION: &str = "version";
const HELP: &str = "help";
const USAGE: &str = help_usage!("printf.md");
const ABOUT: &str = help_about!("printf.md");
const AFTER_HELP: &str = help_section!("after help", "printf.md");

mod options {
    pub const FORMATSTRING: &str = "FORMATSTRING";
    pub const ARGUMENT: &str = "ARGUMENT";
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args = args.collect_ignore();
    let matches = uu_app().get_matches_from(args);

    let format_string = matches
        .get_one::<String>(options::FORMATSTRING)
        .ok_or_else(|| UUsageError::new(1, "missing operand"))?;
    let values: Vec<String> = match matches.get_many::<String>(options::ARGUMENT) {
        Some(s) => s.map(|s| s.to_string()).collect(),
        None => vec![],
    };

    printf(format_string, &values[..])?;
    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .allow_hyphen_values(true)
        .version(crate_version!())
        .about(ABOUT)
        .after_help(AFTER_HELP)
        .override_usage(format_usage(USAGE))
        .disable_help_flag(true)
        .disable_version_flag(true)
        .arg(
            Arg::new(HELP)
                .long(HELP)
                .help("Print help information")
                .action(ArgAction::Help),
        )
        .arg(
            Arg::new(VERSION)
                .long(VERSION)
                .help("Print version information")
                .action(ArgAction::Version),
        )
        .arg(Arg::new(options::FORMATSTRING))
        .arg(Arg::new(options::ARGUMENT).action(ArgAction::Append))
}
