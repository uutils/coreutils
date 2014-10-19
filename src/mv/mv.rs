#![crate_name = "mv"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Sokovikov Evgeniy  <skv-headless@yandex.ru>
 *
 * For the full copyright and license information, please view the LICENSE file
 * that was distributed with this source code.
 */

#![feature(macro_rules)]

extern crate getopts;

use std::io::fs;

use getopts::{
    getopts,
    optflag,
    usage,
};

static NAME: &'static str = "mv";
static VERSION:  &'static str = "0.0.1";

#[path = "../common/util.rs"]
mod util;

pub fn uumain(args: Vec<String>) -> int {
    let opts = [
        optflag("h", "help", "display this help and exit"),
        optflag("V", "version", "output version information and exit"),
        ];
    let matches = match getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(f) => crash!(1, "Invalid options\n{}", f)
    };

    let progname = &args[0];
    let usage = usage("Move SOURCE to DEST", opts);

    if matches.opt_present("version") {
        println!("{}", VERSION);
        return 0;
    }

    if matches.opt_present("help") {
        help(progname.as_slice(), usage.as_slice());
        return 0;
    }

    let source = if matches.free.len() < 1 {
        println!("error: Missing SOURCE argument. Try --help.");
        return 1;
    } else {
        Path::new(matches.free[0].as_slice())
    };

    let dest = if matches.free.len() < 2 {
        println!("error: Missing DEST argument. Try --help.");
        return 1;
    } else {
        Path::new(matches.free[1].as_slice())
    };

    mv(source, dest)
}

fn mv(source: Path, dest: Path) -> int {
    let io_result = fs::rename(&source, &dest);

    if io_result.is_err() {
        let err = io_result.unwrap_err();
        println!("error: {:s}", err.to_string());
        1
    } else {
        0
    }
}

fn help(progname: &str, usage: &str) {
    let msg = format!("Usage: {0} SOURCE DEST\n  \
                         or:  {0} SOURCE... DIRECTORY\n  \
                       \n\
                       {1}", progname, usage);
    println!("{}", msg);
}

