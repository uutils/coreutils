#![crate_name = "uu_link"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Michael Gehring <mg@ebfe.org>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate getopts;

#[macro_use]
extern crate uucore;

use std::fs::hard_link;
use std::io::Write;
use std::path::Path;

static NAME: &'static str = "link";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = getopts::Options::new();

    opts.optflag("h", "help", "display this help and exit");
    opts.optflag("V", "version", "output version information and exit");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(err) => panic!("{}", err),
    };

    if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
        return 0;
    }

    if matches.opt_present("help") || matches.free.len() != 2 {
        let msg = format!("{0} {1}

Usage:
  {0} [OPTIONS] FILE1 FILE2

Create a link named FILE2 to FILE1.", NAME, VERSION);

        println!("{}", opts.usage(&msg));
        if matches.free.len() != 2 {
            return 1;
        }
        return 0;
    }

    let old = Path::new(&matches.free[0]);
    let new = Path::new(&matches.free[1]);

    match hard_link(old, new) {
        Ok(_) => 0,
        Err(err) => {
            show_error!("{}", err);
            1
        }
    }
}
