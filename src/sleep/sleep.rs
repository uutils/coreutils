#![crate_name = "sleep"]
#![feature(rustc_private)]

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
use std::thread::sleep_ms;
use std::u32::MAX as U32_MAX;

#[path = "../common/util.rs"]
#[macro_use]
mod util;

#[path = "../common/time.rs"]
mod time;

static NAME: &'static str = "sleep";

pub fn uumain(args: Vec<String>) -> i32 {
    let opts = [
        getopts::optflag("h", "help", "display this help and exit"),
        getopts::optflag("V", "version", "output version information and exit")
    ];
    let matches = match getopts::getopts(&args[1..], &opts) {
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
        println!("  {0} NUMBER[SUFFIX]", &args[0][..]);
        println!("or");
        println!("  {0} OPTION", &args[0][..]);
        println!("");
        println!("{}", getopts::usage("Pause for NUMBER seconds.  SUFFIX may be 's' for seconds (the default),
'm' for minutes, 'h' for hours or 'd' for days.  Unlike most implementations
that require NUMBER be an integer, here NUMBER may be an arbitrary floating
point number.  Given two or more arguments, pause for the amount of time
specified by the sum of their values.", &opts));
    } else if matches.opt_present("version") {
        println!("sleep 1.0.0");
    } else if matches.free.is_empty() {
        show_error!("missing an argument");
        show_error!("for help, try '{0} --help'", &args[0][..]);
        return 1;
    } else {
        sleep(matches.free);
    }

    0
}

fn sleep(args: Vec<String>) {
    let sleep_time = args.iter().fold(0.0, |result, arg|
        match time::from_str(&arg[..]) {
            Ok(m) => m + result,
            Err(f) => crash!(1, "{}", f),
        });

    let sleep_dur = if sleep_time > (U32_MAX as f64) { 
        U32_MAX
    } else { 
        (1000.0 * sleep_time) as u32
    };
    sleep_ms(sleep_dur);
}


