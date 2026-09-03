// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use clap::{Arg, ArgAction, Command};
use std::ffi::OsString;
use std::io::{Write, stdout};
use std::ops::ControlFlow;
use uucore::display::Quotable;
use uucore::error::{FromIo, UError, UResult, UUsageError};
use uucore::format::{
    FormatArgument, FormatArguments, FormatError, FormatItem, parse_spec_and_escape,
};
use uucore::translate;
use uucore::{format_usage, os_str_as_bytes, show_warning};

mod diagnostics;

const VERSION: &str = "version";
const HELP: &str = "help";

mod options {
    pub const FORMAT: &str = "FORMAT";
    pub const ARGUMENT: &str = "ARGUMENT";
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let result = print_formatted(args);

    // A format without a trailing newline leaves the output sitting in the
    // buffer, so a failed write is only visible once it is flushed. Without
    // this the data would be dropped while printf still reported success.
    // A broken pipe is how a downstream reader normally ends a stream, so it
    // is left alone; and a failure already reported is not repeated.
    if result.is_ok()
        && let Err(e) = stdout().flush()
        && e.kind() != std::io::ErrorKind::BrokenPipe
    {
        return Err(e).map_err_context(|| translate!("common-write-error"));
    }
    result
}

fn print_formatted(args: impl uucore::Args) -> UResult<()> {
    let args: Vec<OsString> = args.collect();
    // Kept for the caret in format diagnostics, which needs the format as typed.
    let diag_args = uucore::diagnostics::capture(&args);
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

    // A parse error is rendered against the argument list when stderr is a
    // terminal; the plain one-line message is kept anywhere else.
    let raise = |error: FormatError| -> Box<dyn UError> {
        uucore::diagnostics::error_after_report(diag_args.as_deref(), error, |args, error| {
            diagnostics::render(args, format, error)
        })
    };

    let mut format_seen = false;
    // Parse and process the format string
    let mut args = FormatArguments::new(&values);
    for item in parse_spec_and_escape(format) {
        if let Ok(FormatItem::Spec(_)) = item {
            format_seen = true;
        }
        match item.map_err(&raise)?.write(stdout(), &mut args)? {
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
            match item.map_err(&raise)?.write(stdout(), &mut args)? {
                ControlFlow::Continue(()) => {}
                ControlFlow::Break(()) => return Ok(()),
            }
        }
        args.start_next_batch();
    }

    Ok(())
}

pub fn uu_app() -> Command {
    Command::new("printf")
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
