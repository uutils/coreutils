#![crate_name = "groups"]
#![feature(rustc_private)]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Alan Andrade <alan.andradec@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 *
 */
extern crate getopts;

use c_types::{get_pw_from_args, group};
use getopts::{getopts, optflag, usage};
use std::io::Write;

#[path = "../common/util.rs"] #[macro_use]  mod util;
#[path = "../common/c_types.rs"] mod c_types;

static NAME: &'static str = "groups";
static VERSION: &'static str = "1.0.0";

pub fn uumain(args: Vec<String>) -> i32 {
    let program = args[0].clone();

    let options = [
        optflag("h", "help", "display this help menu and exit"),
        optflag("V", "version", "display version information and exit")
    ];

    let matches = match getopts(&args[1..], &options) {
        Ok(m) => { m },
        Err(f) => {
            show_error!("{}", f);
            return 1;
        }
    };

    if matches.opt_present("version") {
        println!("{} v{}", NAME, VERSION);
    } else if matches.opt_present("help") {
        print!("{} v{}\n\n\
                Usage:\n  \
                  {} [OPTION]... [USER]...\n\n\
                {}", NAME, VERSION, program, usage("Prints the groups a user is in to standard output.", &options));
    } else {
        group(get_pw_from_args(&matches.free), true);
    }

    0
}
