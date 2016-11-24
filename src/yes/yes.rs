#![crate_name = "uu_yes"]

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

#[macro_use]
extern crate uucore;

use getopts::Options;
use std::io::Write;

static NAME: &'static str = "yes";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = Options::new();

    opts.optflag("h", "help", "display this help and exit");
    opts.optflag("V", "version", "output version information and exit");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => crash!(1, "invalid options\n{}", f)
    };
    if matches.opt_present("help") {
        println!("{} {}", NAME, VERSION);
        println!("");
        println!("Usage:");
        println!("  {0} [STRING]... [OPTION]...", NAME);
        println!("");
        print!("{}", opts.usage("Repeatedly output a line with all specified STRING(s), or 'y'."));
        return 0;
    }
    if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
        return 0;
    }
    let string = if matches.free.is_empty() {
        "y".to_owned()
    } else {
        matches.free.join(" ")
    };

    exec(&string[..]);

    0
}

pub fn exec(string: &str) {
    while pipe_println!("{}", string) { }
}
