#![crate_name = "pwd"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Derek Chiang <derekchiang93@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

#![feature(macro_rules)]

extern crate getopts;
extern crate libc;

use std::io::print;

#[path = "../common/util.rs"]
mod util;

static NAME: &'static str = "pwd";
static VERSION: &'static str = "1.0.0";

pub fn uumain(args: Vec<String>) -> int {
    let program = args[0].clone();
    let opts = [
        getopts::optflag("", "help", "display this help and exit"),
        getopts::optflag("", "version", "output version information and exit"),
    ];

    let matches = match getopts::getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(f) => {
            crash!(1, "Invalid options\n{}", f)
        }
    };

    if matches.opt_present("help") {
        println!("pwd {}", VERSION);
        println!("");
        println!("Usage:");
        println!("  {0:s} [OPTION] NAME...", program);
        println!("");
        print(getopts::usage("Print the full filename of the current working directory.", opts).as_slice());
    } else if matches.opt_present("version") {
        println!("pwd version: {}", VERSION);

        return 0;
    } else {
        let cwd = std::os::getcwd();
        println!("{}", cwd.display());

        return 0;
    }

    0
}
