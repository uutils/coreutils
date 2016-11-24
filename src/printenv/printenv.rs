#![crate_name = "uu_printenv"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Jordi Boggiano <j.boggiano@seld.be>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

/* last synced with: printenv (GNU coreutils) 8.13 */

extern crate getopts;

#[macro_use]
extern crate uucore;

use std::io::Write;
use std::env;

static NAME: &'static str = "printenv";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = getopts::Options::new();
    opts.optflag("0", "null", "end each output line with 0 byte rather than newline");
    opts.optflag("h", "help", "display this help and exit");
    opts.optflag("V", "version", "output version information and exit");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            crash!(1, "Invalid options\n{}", f)
        }
    };
    if matches.opt_present("help") {
        let msg = format!("{0} {1}

Usage:
  {0} [VARIABLE]... [OPTION]...

Prints the given environment VARIABLE(s), otherwise prints them all.", NAME, VERSION);
        print!("{}", opts.usage(&msg)); 
        return 0;
    }
    if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
        return 0;
    }
    let mut separator = "\n";
    if matches.opt_present("null") {
        separator = "\x00";
    };

    exec(matches.free, separator);

    0
}

pub fn exec(args: Vec<String>, separator: &str) {
    if args.is_empty() {
        for (env_var, value) in env::vars() {
            print!("{}={}{}", env_var, value, separator);
        }
        return;
    }

    for env_var in &args {
        if let Ok(var) = env::var(env_var) {
            print!("{}{}", var, separator);
        }
    }
}
