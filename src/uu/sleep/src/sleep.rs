//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Alex Lyon <arcterus@mail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

#[macro_use]
extern crate uucore;

use std::thread;
use std::time::Duration;

use clap::{App, Arg};

static VERSION: &str = env!("CARGO_PKG_VERSION");
static ABOUT: &str = "Pause for NUMBER seconds.";
static LONG_HELP: &str = "Pause for NUMBER seconds.  SUFFIX may be 's' for seconds (the default),
'm' for minutes, 'h' for hours or 'd' for days.  Unlike most implementations
that require NUMBER be an integer, here NUMBER may be an arbitrary floating
point number.  Given two or more arguments, pause for the amount of time
specified by the sum of their values.";

mod options {
    pub const NUMBER: &str = "NUMBER";
}

fn get_usage() -> String {
    format!("{0} [NUMBER]<SUFFIX> \n  or\n    {0} [OPTION]", executable!())
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let usage = get_usage();

    let matches = App::new(executable!())
        .version(VERSION)
        .about(ABOUT)
        .usage(&usage[..])
        .after_help(LONG_HELP)
        .arg(
            Arg::with_name(options::NUMBER)
                .long(options::NUMBER)
                .help("pause for NUMBER seconds")
                .value_name("DURATION")
                .index(1)
                .multiple(true)
        ).get_matches_from(args);
    
    if let Some(values) = matches.values_of(options::NUMBER) {
        let numbers = values.collect();
        sleep(numbers);
    }
    
    0
}

fn sleep(args: Vec<&str>) {
    let sleep_dur =
        args.iter().fold(
            Duration::new(0, 0),
            |result, arg| match uucore::parse_time::from_str(&arg[..]) {
                Ok(m) => m + result,
                Err(f) => crash!(1, "{}", f),
            },
        );

    thread::sleep(sleep_dur);
}
