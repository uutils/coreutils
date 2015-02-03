#![crate_name = "yes"]
#![feature(collections, core, io, libc, rustc_private)]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Jordi Boggiano <j.boggiano@seld.be>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

/* last synced with: yes (GNU coreutils) 8.13 */

extern crate getopts;
extern crate libc;

use std::old_io::print;
use std::borrow::IntoCow;

#[path = "../common/util.rs"]
#[macro_use]
mod util;

static NAME: &'static str = "yes";

pub fn uumain(args: Vec<String>) -> isize {
    let program = &args[0];
    let opts = [
        getopts::optflag("h", "help", "display this help and exit"),
        getopts::optflag("V", "version", "output version information and exit"),
    ];
    let matches = match getopts::getopts(args.tail(), &opts) {
        Ok(m) => m,
        Err(f) => {
            crash!(1, "invalid options\n{}", f)
        }
    };
    if matches.opt_present("help") {
        println!("yes 1.0.0");
        println!("");
        println!("Usage:");
        println!("  {0} [STRING]... [OPTION]...", program);
        println!("");
        print(&getopts::usage("Repeatedly output a line with all specified STRING(s), or 'y'.", &opts)[]);
        return 0;
    }
    if matches.opt_present("version") {
        println!("yes 1.0.0");
        return 0;
    }
    let string = if matches.free.is_empty() {
        "y".into_cow()
    } else {
        matches.free.connect(" ").into_cow()
    };

    exec(&string[]);

    0
}

pub fn exec(string: &str) {
    while pipe_println!("{}", string) { }
}
