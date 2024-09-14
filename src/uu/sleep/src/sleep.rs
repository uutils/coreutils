// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::thread;
use std::time::Duration;

use uucore::{
    error::{UResult, USimpleError, UUsageError},
    format_usage, help_about, help_section, help_usage, show_error,
};

use clap::{crate_version, Arg, ArgAction, Command};
use fundu::{DurationParser, ParseError, SaturatingInto};

static ABOUT: &str = help_about!("sleep.md");
const USAGE: &str = help_usage!("sleep.md");
static AFTER_HELP: &str = help_section!("after help", "sleep.md");

mod options {
    pub const NUMBER: &str = "NUMBER";
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().try_get_matches_from(args)?;

    let numbers = matches
        .get_many::<String>(options::NUMBER)
        .ok_or_else(|| {
            USimpleError::new(
                1,
                format!(
                    "missing operand\nTry '{} --help' for more information.",
                    uucore::execution_phrase()
                ),
            )
        })?
        .map(|s| s.as_str())
        .collect::<Vec<_>>();

    sleep(&numbers)
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .after_help(AFTER_HELP)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::NUMBER)
                .help("pause for NUMBER seconds")
                .value_name(options::NUMBER)
                .action(ArgAction::Append),
        )
}

fn sleep(args: &[&str]) -> UResult<()> {
    let mut arg_error = false;

    use fundu::TimeUnit::{Day, Hour, Minute, Second};
    let parser = DurationParser::with_time_units(&[Second, Minute, Hour, Day]);

    let sleep_dur = args
        .iter()
        .filter_map(|input| match parser.parse(input.trim()) {
            Ok(duration) => Some(duration),
            Err(error) => {
                arg_error = true;

                let reason = match error {
                    ParseError::Empty if input.is_empty() => "Input was empty".to_string(),
                    ParseError::Empty => "Found only whitespace in input".to_string(),
                    ParseError::Syntax(pos, description)
                    | ParseError::TimeUnit(pos, description) => {
                        format!("{description} at position {}", pos.saturating_add(1))
                    }
                    ParseError::NegativeExponentOverflow | ParseError::PositiveExponentOverflow => {
                        "Exponent was out of bounds".to_string()
                    }
                    ParseError::NegativeNumber => "Number was negative".to_string(),
                    error => error.to_string(),
                };
                show_error!("invalid time interval '{input}': {reason}");

                None
            }
        })
        .fold(Duration::ZERO, |acc, n| {
            acc.saturating_add(SaturatingInto::<std::time::Duration>::saturating_into(n))
        });

    if arg_error {
        return Err(UUsageError::new(1, ""));
    };
    thread::sleep(sleep_dur);
    Ok(())
}
