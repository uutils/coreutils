#![feature(macro_rules)]
#![crate_id(name="basename", vers="1.0.0", author="Jimmy Lu")]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Jimmy Lu <jimmy.lu.2011@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate getopts;
extern crate libc;

use std::io::{print, println};
use std::os;
use std::str::StrSlice;

#[path = "../common/util.rs"]
mod util;

static NAME: &'static str = "basename";
static VERSION: &'static str = "1.0.0";

fn main() {
    let args = os::args();
    let program = strip_dir(&args[ 0 ].clone());

    //
    // Argument parsing
    //
    let opts = ~[
        getopts::optflag("h", "help", "display this help and exit"),
        getopts::optflag("V", "version", "output version information and exit"),
    ];

    let matches = match getopts::getopts(args.tail(), opts) {
        Ok(m)  => m,
        Err(f) => crash!(1, "Invalid options\n{}", f.to_err_msg())
    };

    if matches.opt_present("help") {
        println!("Usage: {0:s} NAME [SUFFIX]", program);
        println!("  or: {0:s} OPTION", program);
        println!("Print NAME with any leading directory components removed.");
        println!("If specified, also remove a trailing SUFFIX.");

        print(getopts::usage("", opts));

        return;
    }

    if matches.opt_present("version") {
        println(program + " " + VERSION);
        return;
    }

    // too few arguments
    if args.len() < 2 {
        println(program + ": missing operand");
        println("Try `" + program + " --help' for more information.");
        return;
    }
    // too many arguments
    else if args.len() > 3 {
        println(program + ": extra operand `" + args[ 3 ] + "'");
        println("Try `" + program + " --help' for more information.");
        return;
    }

    //
    // Main Program Processing
    //

    let fullname = args[ 1 ].clone();

    let mut name = strip_dir(&fullname);

    if args.len() > 2 {
        let suffix = args[ 2 ].clone();
        name = strip_suffix(&name, &suffix);
    }

    println(name);
}

fn strip_dir(fullname: &~str) -> ~str {
    let mut name = StrBuf::new();

    for c in fullname.chars().rev() {
        if c == '/' || c == '\\' {
            break;
        }
        name.push_char(c);
    }

    return name.as_slice().chars().rev().collect();
}

fn strip_suffix(name: &~str, suffix: &~str) -> ~str {
    if name == suffix {
        return name.clone();
    }

    if name.ends_with(*suffix) {
        return name.slice_to(name.len() - suffix.len()).into_owned();
    }

    return name.clone();
}
