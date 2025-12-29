// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use clap::{Arg, ArgAction, Command};
use std::ffi::OsString;
use std::io::stdout;
use std::ops::ControlFlow;
use uucore::display::Quotable;
use uucore::error::{UResult, UUsageError};
use uucore::format::{FormatArgument, FormatArguments, FormatItem, parse_spec_and_escape};
use uucore::translate;
use uucore::{format_usage, os_str_as_bytes, show_warning};

const VERSION: &str = "version";
const HELP: &str = "help";

mod options {
    pub const FORMAT: &str = "FORMAT";
    pub const ARGUMENT: &str = "ARGUMENT";
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;

    let format = matches
        .get_one::<OsString>(options::FORMAT)
        .ok_or_else(|| UUsageError::new(1, translate!("printf-error-missing-operand")))?;
    let format = os_str_as_bytes(format)?;

    let values: Vec<_> = match matches.get_many::<OsString>(options::ARGUMENT) {
        Some(s) => s
            .map(|os_string| FormatArgument::Unparsed(os_string.to_owned()))
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
        }
    }
    args.start_next_batch();

    // Without format specs in the string, the iter would not consume any args,
    // leading to an infinite loop. Thus, we exit early.
    if !format_seen {
        if !args.is_exhausted() {
            let Some(FormatArgument::Unparsed(arg_str)) = args.peek_arg() else {
                unreachable!("All args are transformed to Unparsed")
            };
            show_warning!(
                "{}",
                translate!(
                    "printf-warning-ignoring-excess-arguments",
                    "arg" => arg_str.quote()
                )
            );
        }
        return Ok(());
    }

    while !args.is_exhausted() {
        for item in parse_spec_and_escape(format) {
            match item?.write(stdout(), &mut args)? {
                ControlFlow::Continue(()) => {}
                ControlFlow::Break(()) => return Ok(()),
            }
        }
        args.start_next_batch();
    }

    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .allow_hyphen_values(true)
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .about(translate!("printf-about"))
        .after_help(translate!("printf-after-help"))
        .override_usage(format_usage(&translate!("printf-usage")))
        .disable_help_flag(true)
        .disable_version_flag(true)
        .arg(
            Arg::new(HELP)
                .long(HELP)
                .help(translate!("printf-help-help"))
                .action(ArgAction::Help),
        )
        .arg(
            Arg::new(VERSION)
                .long(VERSION)
                .help(translate!("printf-help-version"))
                .action(ArgAction::Version),
        )
        .arg(Arg::new(options::FORMAT).value_parser(clap::value_parser!(OsString)))
        .arg(
            Arg::new(options::ARGUMENT)
                .action(ArgAction::Append)
                .value_parser(clap::value_parser!(OsString)),
        )
}
