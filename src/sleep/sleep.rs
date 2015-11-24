#![crate_name = "sleep"]

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

use std::io::Write;
use std::thread::{self};
use std::time::Duration;
use std::u32::MAX as U32_MAX;

#[path = "../common/util.rs"]
#[macro_use]
mod util;

#[path = "../common/parse_time.rs"]
mod parse_time;

static NAME: &'static str = "sleep";
static VERSION: &'static str = "1.0.0";

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

fn sleep(args: Vec<String>) {
    let sleep_time = args.iter().fold(0.0, |result, arg|
        match parse_time::from_str(&arg[..]) {
            Ok(m) => m + result,
            Err(f) => crash!(1, "{}", f),
        });

    let sleep_dur = if sleep_time > (U32_MAX as f64) { 
        U32_MAX
    } else { 
        (1000000.0 * sleep_time) as u32
    };
    thread::sleep(Duration::new(0, sleep_dur));
}

#[allow(dead_code)]
fn main() {
    std::process::exit(uumain(std::env::args().collect()));
}
