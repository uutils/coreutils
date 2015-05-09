#![crate_name = "basename"]
#![feature(rustc_private)]

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

use std::io::Write;
use std::path::{is_separator, PathBuf};

#[path = "../common/util.rs"]
#[macro_use]
mod util;

static NAME: &'static str = "basename";
static VERSION: &'static str = "1.0.0";

pub fn uumain(args: Vec<String>) -> i32 {
    let program = strip_dir(&args[0]);

    //
    // Argument parsing
    //
    let opts = [
        getopts::optflag("h", "help", "display this help and exit"),
        getopts::optflag("V", "version", "output version information and exit"),
    ];

    let matches = match getopts::getopts(&args[1..], &opts) {
        Ok(m)  => m,
        Err(f) => crash!(1, "Invalid options\n{}", f)
    };

    if matches.opt_present("help") {
        println!("Usage: {0} NAME [SUFFIX]", program);
        println!("  or: {0} OPTION", program);
        println!("Print NAME with any leading directory components removed.");
        println!("If specified, also remove a trailing SUFFIX.");

        print!("{}", getopts::usage("", &opts));

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
        return 1;
    }
    // too many arguments
    else if args.len() > 3 {
        println!("{}: extra operand '{}'", program, args[3]);
        println!("Try '{} --help' for more information.", program);
        return 1;
    }

    //
    // Main Program Processing
    //

    let mut name = strip_dir(&args[1]);

    if args.len() > 2 {
        let suffix = args[2].clone();
        name = strip_suffix(name.as_ref(), suffix.as_ref());
    }

    println!("{}", name);

    0
}

fn strip_dir(fullname: &str) -> String {
    // Remove all platform-specific path separators from the end
    let mut path: String = fullname.chars().rev().skip_while(|&ch| is_separator(ch)).collect();

    // Undo reverse
    path = path.chars().rev().collect();

    // Convert to path buffer and get last path component
    let pb = PathBuf::from(path);
    match pb.components().last() {
        Some(c) => c.as_os_str().to_str().unwrap().to_string(),
        None => "".to_string()
    }
}

fn strip_suffix(name: &str, suffix: &str) -> String {
    if name == suffix {
        return name.to_string();
    }

    if name.ends_with(suffix) {
        return name[..name.len() - suffix.len()].to_string();
    }

    name.to_string()
}
