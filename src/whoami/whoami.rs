#![crate_name = "whoami"]
#![feature(rustc_private)]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Jordi Boggiano <j.boggiano@seld.be>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

/* last synced with: whoami (GNU coreutils) 8.21 */

extern crate getopts;
extern crate libc;

use std::io::Write;

#[path = "../common/util.rs"] #[macro_use] mod util;
mod platform;

static NAME: &'static str = "whoami";

pub fn uumain(args: Vec<String>) -> i32 {
    let opts = [
        getopts::optflag("h", "help", "display this help and exit"),
        getopts::optflag("V", "version", "output version information and exit"),
    ];
    let matches = match getopts::getopts(&args[1..], &opts) {
        Ok(m) => m,
        Err(f) => crash!(1, "{}", f),
    };
    if matches.opt_present("help") {
        println!("whoami 1.0.0");
        println!("");
        println!("Usage:");
        println!("  {}", args[0]);
        println!("");
        println!("{}", getopts::usage("print effective userid", &opts));
        return 0;
    }
    if matches.opt_present("version") {
        println!("whoami 1.0.0");
        return 0;
    }

    exec();

    0
}

pub fn exec() {
    unsafe {
        let username = platform::getusername();
        println!("{}", username);
    }
}
