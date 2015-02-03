#![crate_name = "dirname"]
#![feature(collections, core, io, path, rustc_private)]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Derek Chiang <derekchiang93@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate getopts;

use std::old_io::print;

static VERSION: &'static str = "1.0.0";

pub fn uumain(args: Vec<String>) -> isize {
    let program = args[0].clone();
    let opts = [
        getopts::optflag("z", "zero", "separate output with NUL rather than newline"),
        getopts::optflag("", "help", "display this help and exit"),
        getopts::optflag("", "version", "output version information and exit"),
    ];

    let matches = match getopts::getopts(args.tail(), &opts) {
        Ok(m) => m,
        Err(f) => panic!("Invalid options\n{}", f)
    };

    if matches.opt_present("help") {
        println!("dirname {} - strip last component from file name", VERSION);
        println!("");
        println!("Usage:");
        println!("  {0} [OPTION] NAME...", program);
        println!("");
        print(getopts::usage("Output each NAME with its last non-slash component and trailing slashes
removed; if NAME contains no  /'s,  output  '.'  (meaning  the  current
directory).", &opts).as_slice());
        return 0;
    }

    if matches.opt_present("version") {
        println!("dirname version: {}", VERSION);
        return 0;
    }

    let separator = match matches.opt_present("zero") {
        true => "\0",
        false => "\n"
    };

    if !matches.free.is_empty() {
        for path in matches.free.iter() {
            let p = std::path::Path::new(path.clone());
            let d = std::str::from_utf8(p.dirname());
            if d.is_ok() {
                print(d.unwrap());
            }
            print(separator);
        }
    } else {
        println!("{0}: missing operand", program);
        println!("Try '{0} --help' for more information.", program);
        return 1;
    }

    0
}
