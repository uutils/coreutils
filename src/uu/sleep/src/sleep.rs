//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Alex Lyon <arcterus@mail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

use std::thread;
use std::time::Duration;

use uucore::{
    error::{UResult, USimpleError},
    format_usage,
};

use clap::{crate_version, Arg, Command};

static ABOUT: &str = "Pause for NUMBER seconds.";
const USAGE: &str = "\
    {} NUMBER[SUFFIX]...
    {} OPTION";
static LONG_HELP: &str = "Pause for NUMBER seconds.  SUFFIX may be 's' for seconds (the default),
'm' for minutes, 'h' for hours or 'd' for days.  Unlike most implementations
that require NUMBER be an integer, here NUMBER may be an arbitrary floating
point number.  Given two or more arguments, pause for the amount of time
specified by the sum of their values.";

mod options {
    pub const NUMBER: &str = "NUMBER";
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().get_matches_from(args);

    if let Some(values) = matches.values_of(options::NUMBER) {
        let numbers = values.collect::<Vec<_>>();
        return sleep(&numbers);
    }

    Ok(())
}

pub fn uu_app<'a>() -> Command<'a> {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .after_help(LONG_HELP)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::NUMBER)
                .help("pause for NUMBER seconds")
                .value_name(options::NUMBER)
                .index(1)
                .multiple_occurrences(true)
                .required(true),
        )
}

fn sleep(args: &[&str]) -> UResult<()> {
    let sleep_dur =
        args.iter().try_fold(
            Duration::new(0, 0),
            |result, arg| match uucore::parse_time::from_str(&arg[..]) {
                Ok(m) => Ok(m + result),
                Err(f) => Err(USimpleError::new(1, f)),
            },
        )?;
    thread::sleep(sleep_dur);
    Ok(())
}
