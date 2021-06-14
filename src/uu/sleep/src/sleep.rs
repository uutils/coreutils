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

use crate::app::get_app;
use crate::app::options;

mod app;

fn get_usage() -> String {
    format!(
        "{0} {1}[SUFFIX]... \n    {0} OPTION",
        executable!(),
        options::NUMBER
    )
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let usage = get_usage();

    let matches = get_app(executable!())
        .usage(&usage[..])
        .get_matches_from(args);

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
