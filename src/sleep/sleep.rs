#![crate_name = "uu_sleep"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Arcterus <arcterus@mail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate getopts;
extern crate libc;

#[macro_use]
extern crate uucore;

use std::io::Write;
use std::thread::{self};
use std::time::Duration;

static NAME: &'static str = "sleep";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = getopts::Options::new();
    opts.optflag("h", "help", "display this help and exit");
    opts.optflag("V", "version", "output version information and exit");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            show_error!("{}", f);
            return 1;
        }
    };

    if matches.opt_present("help") {
        let msg = format!("{0} {1}

Usage:
  {0} NUMBER[SUFFIX]
or
  {0} OPTION

Pause for NUMBER seconds.  SUFFIX may be 's' for seconds (the default),
'm' for minutes, 'h' for hours or 'd' for days.  Unlike most implementations
that require NUMBER be an integer, here NUMBER may be an arbitrary floating
point number.  Given two or more arguments, pause for the amount of time
specified by the sum of their values.", NAME, VERSION);
        print!("{}", opts.usage(&msg));
    } else if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
    } else if matches.free.is_empty() {
        show_error!("missing an argument");
        show_error!("for help, try '{0} --help'", NAME);
        return 1;
    } else {
        sleep(matches.free);
    }

    0
}

fn float_to_duration(dur_f: f64) -> Duration {
    let seconds = dur_f as u64;
    let nanos = ((dur_f - seconds as f64) * 1e9) as u32;
    Duration::new(seconds, nanos)
}

fn sleep(args: Vec<String>) {
    let sleep_time = args.iter().fold(0.0, |result, arg|
        match uucore::parse_time::from_str(&arg[..]) {
            Ok(m) => m + result,
            Err(f) => crash!(1, "{}", f),
        });
    let sleep_dur = float_to_duration(sleep_time);

    thread::sleep(sleep_dur);
}
