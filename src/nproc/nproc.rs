#![crate_name = "nproc"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Michael Gehring <mg@ebfe.org>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate getopts;
extern crate num_cpus;

use std::env;
use std::io::Write;

static NAME : &'static str = "nproc";
static VERSION : &'static str = "0.0.0";

#[path = "../common/util.rs"]
#[macro_use]
mod util;

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = getopts::Options::new();

    opts.optflag("",
                 "all",
                 "print the number of cores available to the system");
    opts.optopt("", "ignore", "ignore up to N cores", "N");
    opts.optflag("h", "help", "display this help and exit");
    opts.optflag("V", "version", "output version information and exit");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(err) => {
            show_error!("{}", err);
            return 1;
        }
    };

    if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
        return 0;
    }

    if matches.opt_present("help") {
        let msg = format!("{0} {1}

Usage:
  {0} [OPTIONS]...

Print the number of cores available to the current process.",
                          NAME,
                          VERSION);

        print!("{}", opts.usage(&msg));
        return 0;
    }

    let mut ignore = match matches.opt_str("ignore") {
        Some(numstr) => match numstr.parse() {
            Ok(num) => num,
            Err(e) => {
                show_error!("\"{}\" is not a valid number: {}", numstr, e);
                return 1;
            }
        },
        None => 0,
    };

    if !matches.opt_present("all") {
        ignore += match env::var("OMP_NUM_THREADS") {
            Ok(threadstr) => match threadstr.parse() {
                Ok(num) => num,
                Err(_) => 0,
            },
            Err(_) => 0,
        };
    }

    let mut cores = num_cpus::get();
    if cores <= ignore {
        cores = 1;
    } else {
        cores -= ignore;
    }
    println!("{}", cores);
    0
}
