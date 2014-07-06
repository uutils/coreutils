#![crate_name = "sleep"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Arcterus <arcterus@mail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

#![feature(macro_rules)]

extern crate getopts;
extern crate libc;

use std::f64;
use std::io::{print, timer};
use std::u64;

#[path = "../common/util.rs"]
mod util;

static NAME: &'static str = "sleep";

pub fn uumain(args: Vec<String>) -> int {
    let program = args.get(0).clone();

    let opts = [
        getopts::optflag("h", "help", "display this help and exit"),
        getopts::optflag("V", "version", "output version information and exit")
    ];
    let matches = match getopts::getopts(args.tail(), opts) {
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
        println!("  {0:s} NUMBER[SUFFIX]", program);
        println!("or");
        println!("  {0:s} OPTION", program);
        println!("");
        print(getopts::usage("Pause for NUMBER seconds.  SUFFIX may be 's' for seconds (the default),
'm' for minutes, 'h' for hours or 'd' for days.  Unlike most implementations
that require NUMBER be an integer, here NUMBER may be an arbitrary floating
point number.  Given two or more arguments, pause for the amount of time
specified by the sum of their values.", opts).as_slice());
    } else if matches.opt_present("version") {
        println!("sleep 1.0.0");
    } else if matches.free.is_empty() {
        show_error!("missing an argument");
        show_error!("for help, try '{0:s} --help'", program);
        return 1;
    } else {
        sleep(matches.free);
    }

    0
}

fn sleep(args: Vec<String>) {
    let sleep_time = args.iter().fold(0.0, |result, arg| {
        let (arg, suffix_time) = match match_suffix(arg.as_slice()) {
            Ok(m) => m,
            Err(f) => {
                crash!(1, "{}", f.to_string())
            }
        };
        let num =
            if suffix_time == 0 {
                0.0
            } else {
                match from_str::<f64>(arg.as_slice()) {
                    Some(m) => m,
                    None => {
                        crash!(1, "Invalid time interval '{}'", arg.to_string())
                    }
                }
            };
        result + num * suffix_time as f64
    });
    timer::sleep(if sleep_time == f64::INFINITY { u64::MAX } else { (sleep_time * 1000.0) as u64 });
}

fn match_suffix(arg: &str) -> Result<(String, int), String> {
    let result = match (arg).char_at_reverse(0) {
        's' | 'S' => 1,
        'm' | 'M' => 60,
        'h' | 'H' => 60 * 60,
        'd' | 'D' => 60 * 60 * 24,
        val => {
            if !val.is_alphabetic() {
                return Ok((arg.to_string(), 1))
            } else if arg == "inf" || arg == "infinity" {
                return Ok(("inf".to_string(), 1))
            } else {
                return Err(format!("Invalid time interval '{}'", arg))
            }
        }
    };
    Ok(((arg).slice_to((arg).len() - 1).to_string(), result))
}
