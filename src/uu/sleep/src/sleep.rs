//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Alex Lyon <arcterus@mail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

use std::thread;
use std::time::Duration;

use uucore::{
    error::{UResult, USimpleError, UUsageError},
    format_usage, help_about, help_section, help_usage, show,
};

use clap::{crate_version, Arg, ArgAction, Command};

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
    let intervals = args.iter().map(|s| match uucore::parse_time::from_str(s) {
        Ok(result) => result,
        Err(err) => {
            arg_error = true;
            show!(USimpleError::new(1, err));
            Duration::new(0, 0)
        }
    });
    let sleep_dur = intervals.fold(Duration::new(0, 0), |acc, n| acc + n);
    if arg_error {
        return Err(UUsageError::new(1, ""));
    };
    thread::sleep(sleep_dur);
    Ok(())
}
