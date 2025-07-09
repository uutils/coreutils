// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{Arg, ArgAction, Command};
use std::collections::HashMap;
use std::thread;
use std::time::Duration;
use uucore::locale::{get_message, get_message_with_args};
use uucore::{
    error::{UResult, USimpleError, UUsageError},
    format_usage,
    parser::parse_time,
    show_error,
};

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
                get_message_with_args(
                    "sleep-error-missing-operand",
                    HashMap::from([(
                        "program".to_string(),
                        uucore::execution_phrase().to_string(),
                    )]),
                ),
            )
        })?
        .map(|s| s.as_str())
        .collect::<Vec<_>>();

    sleep(&numbers)
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .about(get_message("sleep-about"))
        .after_help(get_message("sleep-after-help"))
        .override_usage(format_usage(&get_message("sleep-usage")))
        .infer_long_args(true)
        .arg(
            Arg::new(options::NUMBER)
                .help(get_message("sleep-help-number"))
                .value_name(options::NUMBER)
                .action(ArgAction::Append),
        )
}

fn sleep(args: &[&str]) -> UResult<()> {
    let mut arg_error = false;

    let sleep_dur = args
        .iter()
        .filter_map(|input| match parse_time::from_str(input, true) {
            Ok(duration) => Some(duration),
            Err(error) => {
                arg_error = true;
                show_error!("{error}");
                None
            }
        })
        .fold(Duration::ZERO, |acc, n| acc.saturating_add(n));

    if arg_error {
        return Err(UUsageError::new(1, ""));
    }
    thread::sleep(sleep_dur);
    Ok(())
}
