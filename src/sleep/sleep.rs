#![crate_name = "uu_sleep"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Arcterus <arcterus@mail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

#[macro_use]
extern crate clap;

#[macro_use]
extern crate uucore;

use clap::{App, Arg};
use std::ffi::{OsStr, OsString};
use std::io::Write;
use std::thread;
use std::time::Duration;

const AFTER_HELP: &'static str = "\
SUFFIX may be 's' for seconds (the default), 'm' for minutes, 'h' for hours or
'd' for days.  Unlike most implementations that require NUMBER be an integer,
here NUMBER may be an arbitrary floating point number.  Given two or more
arguments, pause for the amount of time specified by the sum of their values.
";

pub fn uumain(args: Vec<OsString>) -> i32 {
    let matches = App::new(executable!(args))
                          .version(crate_version!())
                          .author("uutils developers (https://github.com/uutils)")
                          .about("Pause for NUMBER seconds.")
                          .after_help(AFTER_HELP)
                          .arg(Arg::with_name("NUMBER[SUFFIX]")
                               .help("The number of seconds to pause")
                               .required(true)
                               .index(1)
                               .multiple(true))
                          .get_matches_from(args);

    sleep(matches.values_of_os("NUMBER[SUFFIX]").unwrap().collect());

    0
}

fn sleep(args: Vec<&OsStr>) {
    let sleep_dur = args.iter().fold(Duration::new(0, 0), |result, arg| {
        match uucore::parse_time::from_str(&arg.to_string_lossy()) {
            Ok(m) => m + result,
            Err(f) => crash!(1, "{}", f),
        }
    });

    thread::sleep(sleep_dur);
}
