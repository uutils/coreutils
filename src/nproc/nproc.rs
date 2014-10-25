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

use std::os;

static NAME : &'static str = "nproc";
static VERSION : &'static str = "0.0.0";

pub fn uumain(args: Vec<String>) -> int {
    let opts = [
        getopts::optflag("h", "help", "display this help and exit"),
        getopts::optflag("V", "version", "output version information and exit"),
    ];

    let matches = match getopts::getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(err) => fail!("{}", err),
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
        print!("{}", getopts::usage("Print the number of cores available.", opts.as_slice()).as_slice());
        if matches.free.is_empty() {
            return 1;
        }
        return 0;
    }

    println!("{}", os::num_cpus());

    return 0
}
