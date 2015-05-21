#![crate_name = "pwd"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Derek Chiang <derekchiang93@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate getopts;
extern crate libc;

use std::env;
use std::io::Write;

#[path = "../common/util.rs"]
#[macro_use]
mod util;

static NAME: &'static str = "pwd";
static VERSION: &'static str = "1.0.0";

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = getopts::Options::new();

    opts.optflag("", "help", "display this help and exit");
    opts.optflag("", "version", "output version information and exit");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            crash!(1, "Invalid options\n{}", f)
        }
    };

    if matches.opt_present("help") {
        let msg = format!("{0} {1}

Usage:
  {0} [OPTION]...

Print the full filename of the current working directory.", NAME, VERSION);
        print!("{}", opts.usage(&msg));
    } else if matches.opt_present("version") {
        println!("{} version: {}", NAME, VERSION);
    } else {
        println!("{}", env::current_dir().unwrap().display());
    }

    0
}
