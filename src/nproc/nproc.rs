#![crate_name = "nproc"]
#![feature(collections, os, rustc_private)]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Michael Gehring <mg@ebfe.org>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate getopts;

use std::os;

static NAME : &'static str = "nproc";
static VERSION : &'static str = "0.0.0";

#[path = "../common/util.rs"]
#[macro_use]
mod util;

pub fn uumain(args: Vec<String>) -> isize {
    let opts = [
        getopts::optflag("", "all", "print the number of cores available to the system"),
        getopts::optopt("", "ignore", "ignore up to N cores", "N"),
        getopts::optflag("h", "help", "display this help and exit"),
        getopts::optflag("V", "version", "output version information and exit"),
    ];

    let matches = match getopts::getopts(args.tail(), &opts) {
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
        println!("{} {}", NAME, VERSION);
        println!("");
        println!("Usage:");
        println!("  {} [OPTIONS] NAME...", NAME);
        println!("");
        print!("{}", getopts::usage("Print the number of cores available to the current process.", &opts));
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
        None => 0
    };

    if !matches.opt_present("all") {
        ignore += match os::getenv("OMP_NUM_THREADS") {
            Some(threadstr) => match threadstr.parse() {
                Ok(num) => num,
                Err(_)=> 0
            },
            None => 0
        };
    }

    let mut cores = os::num_cpus();
    if cores <= ignore {
        cores = 1;
    } else {
        cores -= ignore;
    }
    println!("{}", cores);

    return 0
}
