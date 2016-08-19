#![crate_name = "uu_whoami"]

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

#[macro_use]
extern crate uucore;

use getopts::Options;
use std::io::Write;

mod platform;

static NAME: &'static str = "whoami";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = Options::new();

    opts.optflag("h", "help", "display this help and exit");
    opts.optflag("V", "version", "output version information and exit");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => crash!(1, "{}", f),
    };
    if matches.opt_present("help") {
        println!("{} {}", NAME, VERSION);
        println!("");
        println!("Usage:");
        println!("  {} [OPTIONS]", NAME);
        println!("");
        println!("{}", opts.usage("print effective userid"));
        return 0;
    }
    if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
        return 0;
    }

    exec();

    0
}

pub fn exec() {
    unsafe {
        match platform::getusername() {
            Ok(username) => println!("{}", username),
            Err(err) => match err.raw_os_error() {
                Some(0) | None => crash!(1, "failed to get username"),
                Some(_) => crash!(1, "failed to get username: {}", err),
            }
        }
    }
}
