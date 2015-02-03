#![crate_name = "link"]
#![feature(collections, core, io, path, rustc_private)]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Michael Gehring <mg@ebfe.org>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate getopts;

use std::old_io::fs::link;
use std::path::Path;

#[path="../common/util.rs"]
#[macro_use]
mod util;

static NAME : &'static str = "link";
static VERSION : &'static str = "1.0.0";

pub fn uumain(args: Vec<String>) -> isize {
    let opts = [
        getopts::optflag("h", "help", "display this help and exit"),
        getopts::optflag("V", "version", "output version information and exit"),
    ];

    let matches = match getopts::getopts(args.tail(), &opts) {
        Ok(m) => m,
        Err(err) => panic!("{}", err),
    };

    if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
        return 0;
    }

    if matches.opt_present("help") || matches.free.len() != 2 {
        println!("{} {}", NAME, VERSION);
        println!("");
        println!("Usage:");
        println!("  {} [OPTIONS] FILE1 FILE2", NAME);
        println!("");
        print!("{}", getopts::usage("Create a link named FILE2 to FILE1.", opts.as_slice()).as_slice());
        if matches.free.len() != 2 {
            return 1;
        }
        return 0;
    }

    let old = Path::new(matches.free[0].as_slice());
    let new = Path::new(matches.free[1].as_slice());

    match link(&old, &new) {
        Ok(_) => 0,
        Err(err) => {
            show_error!("{}", err);
            1
        }
    }
}
