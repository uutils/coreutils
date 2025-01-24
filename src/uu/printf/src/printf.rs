// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{crate_version, Arg, ArgAction, Command};
use std::io::stdout;
use std::ops::ControlFlow;
#[cfg(unix)]
use std::os::unix::ffi::{OsStrExt, OsStringExt};
#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
use uucore::error::{UResult, UUsageError};
use uucore::format::{parse_spec_and_escape, FormatArgument, FormatItem};
use uucore::{format_usage, help_about, help_section, help_usage};

const VERSION: &str = "version";
const HELP: &str = "help";
const USAGE: &str = help_usage!("printf.md");
const ABOUT: &str = help_about!("printf.md");
const AFTER_HELP: &str = help_section!("after help", "printf.md");

mod options {
    pub const FORMAT: &str = "FORMAT";
    pub const ARGUMENT: &str = "ARGUMENT";
}
#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().get_matches_from(args);

    let format = matches
        .get_one::<std::ffi::OsString>(options::FORMAT)
        .ok_or_else(|| UUsageError::new(1, "missing operand"))?;

    #[cfg(unix)]
    let format = format.as_bytes();

    #[cfg(windows)]
    let format = as_os_str().as_bytes_lossy();

    let values: Vec<_> = match matches.get_many::<std::ffi::OsString>(options::ARGUMENT) {
        Some(s) => s
            .map(|os_str| {
                let raw_bytes: Vec<u8> = os_str.clone().into_vec();
                #[cfg(unix)]
                {
                    FormatArgument::Unparsed(
                        String::from_utf8(raw_bytes.clone())
                            .unwrap_or_else(|_| raw_bytes.iter().map(|&b| b as char).collect()),
                    )
                }
                #[cfg(windows)]
                FormatArgument::Unparsed(String::from_utf8_lossy(&raw_bytes).into_owned())
            })
            .collect(),
        None => vec![],
    };

    let mut format_seen = false;
    let mut args = values.iter().peekable();

    // Parse and process the format string
    for item in parse_spec_and_escape(format) {
        if let Ok(FormatItem::Spec(_)) = item {
            format_seen = true;
        }
        match item?.write(stdout(), &mut args)? {
            ControlFlow::Continue(()) => {}
            ControlFlow::Break(()) => return Ok(()),
        };
    }

    // Without format specs in the string, the iter would not consume any args,
    // leading to an infinite loop. Thus, we exit early.
    if !format_seen {
        return Ok(());
    }

    while args.peek().is_some() {
        for item in parse_spec_and_escape(format) {
            match item?.write(stdout(), &mut args)? {
                ControlFlow::Continue(()) => {}
                ControlFlow::Break(()) => return Ok(()),
            };
        }
    }
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
        .arg(Arg::new(options::FORMAT).value_parser(clap::value_parser!(std::ffi::OsString)))
        .arg(
            Arg::new(options::ARGUMENT)
                .action(ArgAction::Append)
                .value_parser(clap::value_parser!(std::ffi::OsString)),
        )
}
