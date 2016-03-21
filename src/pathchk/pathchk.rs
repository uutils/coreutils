#![crate_name = "uu_pathchk"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Inokentiy Babushkin <inokentiy.babushkin@googlemail.com>
 *
 * For the full copyright and license information, please view the LICENSE file
 * that was distributed with this source code.
 */

extern crate getopts;

#[macro_use]
extern crate uucore;

use getopts::Options;
use std::io::Write;

enum Mode {
    PosixMost,
    PosixSpecial,
    PosixAll,
    Help,
    Version
}

static NAME: &'static str = "pathchk";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub fn uumain(args: Vec<String>) -> i32 {
    // add options
    let mut opts = Options::new();
    opts.optflag("p", "posix", "check for POSIX systems");
    opts.optflag("P",
                 "posix-special", "check for empty names and leading \"-\"");
    opts.optflag("",
        "portability", "check for all POSIX systems (equivalent to -p -P)");
    opts.optflag("h", "help", "display this help text and exit");
    opts.optflag("V", "version", "output version information and exit");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(e) => { crash!(1, "{}", e) }
    };

    // set working mode
    let mode = if matches.opt_present("version") {
        Mode::Version
    } else if (matches.opt_present("posix") &&
               matches.opt_present("posix-special")) ||
              matches.opt_present("portability") {
        Mode::PosixAll
    } else if matches.opt_present("posix") {
        Mode::PosixMost
    } else if matches.opt_present("posix-special") {
        Mode::PosixSpecial
    } else {
        Mode::Help
    };

    match mode {
        Mode::Help => { help(opts); 0 }
        Mode::Version => { version(); 0 }
        _ => check_path(mode, matches.free)
    }
}

fn help(opts: Options) {
    let msg = format!("Usage: {} [OPTION]... NAME...\n\n\
    Diagnose invalid or unportable file names.", NAME);

    print!("{}", opts.usage(&msg));
}

fn version() {
    println!("{} {}", NAME, VERSION);
}

fn check_path(mode: Mode, paths: Vec<String>) -> i32 {
    0 // TODO: implement
}
