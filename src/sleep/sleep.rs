#![crate_name = "sleep"]
#![feature(collections, core, io, libc, rustc_private, std_misc)]

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

use std::f64;
use std::old_io::{print, timer};
use std::time::duration::{self, Duration};

#[path = "../common/util.rs"]
#[macro_use]
mod util;

#[path = "../common/time.rs"]
mod time;

static NAME: &'static str = "sleep";

pub fn uumain(args: Vec<String>) -> isize {
    let program = args[0].clone();

    let opts = [
        getopts::optflag("h", "help", "display this help and exit"),
        getopts::optflag("V", "version", "output version information and exit")
    ];
    let matches = match getopts::getopts(args.tail(), &opts) {
        Ok(m) => m,
        Err(f) => {
            show_error!("{}", f);
            return 1;
        }
    };

    if matches.opt_present("help") {
        println!("sleep 1.0.0");
        println!("");
        println!("Usage:");
        println!("  {0} NUMBER[SUFFIX]", program);
        println!("or");
        println!("  {0} OPTION", program);
        println!("");
        print(getopts::usage("Pause for NUMBER seconds.  SUFFIX may be 's' for seconds (the default),
'm' for minutes, 'h' for hours or 'd' for days.  Unlike most implementations
that require NUMBER be an integer, here NUMBER may be an arbitrary floating
point number.  Given two or more arguments, pause for the amount of time
specified by the sum of their values.", &opts).as_slice());
    } else if matches.opt_present("version") {
        println!("sleep 1.0.0");
    } else if matches.free.is_empty() {
        show_error!("missing an argument");
        show_error!("for help, try '{0} --help'", program);
        return 1;
    } else {
        sleep(matches.free);
    }

    0
}

fn sleep(args: Vec<String>) {
    let sleep_time = args.iter().fold(0.0, |result, arg| {
        let num = match time::from_str(arg.as_slice()) {
            Ok(m) => m,
            Err(f) => {
                crash!(1, "{}", f)
            }
        };
        result + num
    });
    let sleep_dur = if sleep_time == f64::INFINITY { 
        duration::MAX 
    } else { 
        Duration::seconds(sleep_time as i64)
    };
    timer::sleep(sleep_dur);
}
