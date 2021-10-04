//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Alex Lyon <arcterus@mail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

use std::thread;
use std::time::Duration;

use uucore::error::{UResult, USimpleError};

use clap::{crate_version, App, Arg};

static ABOUT: &str = "Pause for NUMBER seconds.";
static LONG_HELP: &str = "Pause for NUMBER seconds.  SUFFIX may be 's' for seconds (the default),
'm' for minutes, 'h' for hours or 'd' for days.  Unlike most implementations
that require NUMBER be an integer, here NUMBER may be an arbitrary floating
point number.  Given two or more arguments, pause for the amount of time
specified by the sum of their values.";

mod options {
    pub const NUMBER: &str = "NUMBER";
}

fn usage() -> String {
    format!(
        "{0} {1}[SUFFIX]... \n    {0} OPTION",
        uucore::execution_phrase(),
        options::NUMBER
    )
}

#[uucore_procs::gen_uumain]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let usage = usage();

    let matches = uu_app().usage(&usage[..]).get_matches_from(args);

    if let Some(values) = matches.values_of(options::NUMBER) {
        let numbers = values.collect();
        return sleep(numbers);
    }

    Ok(())
}

pub fn uu_app() -> App<'static, 'static> {
    App::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .after_help(LONG_HELP)
        .arg(
            Arg::with_name(options::NUMBER)
                .long(options::NUMBER)
                .help("pause for NUMBER seconds")
                .value_name(options::NUMBER)
                .index(1)
                .multiple(true)
                .required(true),
        )
}

fn sleep(args: Vec<&str>) -> UResult<()> {
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
