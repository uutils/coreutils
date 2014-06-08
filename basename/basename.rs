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

#[allow(dead_code)]
fn main() { os::set_exit_status(uumain(os::args())); }

pub fn uumain(args: Vec<String>) -> int {
    let program = strip_dir(args.get(0).as_slice());

    //
    // Argument parsing
    //
    let opts = [
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

        print(getopts::usage("", opts).as_slice());

        return 0;
    }

    if matches.opt_present("version") {
        println!("{} {}", program, VERSION);
        return 0;
    }

    // too few arguments
    if args.len() < 2 {
        println!("{}: {}", program, "missing operand");
        println!("Try '{} --help' for more information.", program);
        return 0;
    }
    // too many arguments
    else if args.len() > 3 {
        println!("{}: extra operand '{}'", program, args.get(3));
        println!("Try '{} --help' for more information.", program);
        return 0;
    }

    //
    // Main Program Processing
    //

    let fullname = args.get(1);

    let mut name = strip_dir(fullname.as_slice());

    if args.len() > 2 {
        let suffix = args.get(2).clone();
        name = strip_suffix(name.as_slice(), suffix.as_slice());
    }

    println(name.as_slice());

    return 0;
}

fn strip_dir(fullname: &str) -> String {
    let mut name = String::new();

    for c in fullname.chars().rev() {
        if c == '/' || c == '\\' {
            break;
        }
        name.push_char(c);
    }

    return name.as_slice().chars().rev().collect();
}

fn strip_suffix(name: &str, suffix: &str) -> String {
    if name == suffix {
        return name.into_string();
    }

    if name.ends_with(suffix) {
        return name.slice_to(name.len() - suffix.len()).into_string();
    }

    return name.into_string();
}
