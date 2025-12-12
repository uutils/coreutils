// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use clap::{Arg, ArgAction, Command};
use std::ffi::OsString;
use std::io::{Write, stdout};
use std::ops::ControlFlow;
use uucore::error::{UResult, USimpleError, UUsageError};
use uucore::format::{FormatArgument, FormatArguments, FormatItem, parse_spec_and_escape};
use uucore::signals::stdout_was_closed;
use uucore::translate;
use uucore::{format_usage, os_str_as_bytes, show_warning};

// Capture stdout state at process startup (before Rust's runtime may reopen closed fds)
uucore::init_sigpipe_capture!();

const VERSION: &str = "version";
const HELP: &str = "help";

/// Check for stdout write errors after output has been written.
/// This handles both /dev/full (flush error) and closed stdout (reopened as /dev/null).
fn check_stdout_errors() -> UResult<()> {
    // Check for stdout write errors (e.g., /dev/full)
    if let Err(e) = stdout().flush() {
        return Err(USimpleError::new(1, e.to_string()));
    }

    // Check if stdout was closed before Rust's runtime reopened it as /dev/null.
    // Uses the early-capture mechanism from init_sigpipe_capture!() to detect this
    // at process startup, not at runtime (which would incorrectly trigger on
    // legitimate redirects to /dev/null).
    if stdout_was_closed() {
        return Err(USimpleError::new(1, "write error"));
    }

    Ok(())
}

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
                    "arg" => arg_str.to_string_lossy()
                )
            );
        }
        // Check for write errors if we wrote any output
        if !format.is_empty() {
            check_stdout_errors()?;
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

    // Check for write errors (format is always non-empty here since we would have
    // returned early above if !format_seen and there were no format specs)
    check_stdout_errors()
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
