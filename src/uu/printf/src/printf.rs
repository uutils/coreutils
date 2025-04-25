// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use clap::{Arg, ArgAction, Command};
use std::io::stdout;
use std::ops::ControlFlow;
use uucore::error::{UResult, UUsageError};
use uucore::format::{FormatArgument, FormatArguments, FormatItem, parse_spec_and_escape};
use uucore::{format_usage, help_about, help_section, help_usage, os_str_as_bytes, show_warning};

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
    let format = os_str_as_bytes(format)?;

    let values: Vec<_> = match matches.get_many::<std::ffi::OsString>(options::ARGUMENT) {
        // FIXME: use os_str_as_bytes once FormatArgument supports Vec<u8>
        Some(s) => s
            .map(|os_string| {
                FormatArgument::Unparsed(std::ffi::OsStr::to_string_lossy(os_string).to_string())
            })
            .collect(),
        None => vec![],
    };

    let mut format_seen = false;
    // Parse and process the format string
    let mut args = FormatArguments::new(&values);
    for item in parse_spec_and_escape(format) {
        if let Ok(FormatItem::Spec(_)) = item {
            format_seen = true;
        }
        match item?.write(stdout(), &mut args)? {
            ControlFlow::Continue(()) => {}
            ControlFlow::Break(()) => return Ok(()),
        };
    }
    args.start_next_batch();

    // Without format specs in the string, the iter would not consume any args,
    // leading to an infinite loop. Thus, we exit early.
    if !format_seen {
        if !args.is_exhausted() {
            let Some(FormatArgument::Unparsed(arg_str)) = args.peek_arg() else {
                unreachable!("All args are transformed to Unparsed")
            };
            show_warning!("ignoring excess arguments, starting with '{arg_str}'");
        }
        return Ok(());
    }

    while !args.is_exhausted() {
        for item in parse_spec_and_escape(format) {
            match item?.write(stdout(), &mut args)? {
                ControlFlow::Continue(()) => {}
                ControlFlow::Break(()) => return Ok(()),
            };
        }
        args.start_next_batch();
    }

    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .allow_hyphen_values(true)
        .version(uucore::crate_version!())
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
