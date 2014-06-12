#![crate_id(name="dirname", vers="1.0.0", author="Derek Chiang")]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Derek Chiang <derekchiang93@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate getopts;

use std::os;
use std::io::print;

static VERSION: &'static str = "1.0.0";

#[allow(dead_code)]
fn main() { os::set_exit_status(uumain(os::args())); }

pub fn uumain(args: Vec<String>) -> int {
    let program = args.get(0).clone();
    let opts = [
        getopts::optflag("z", "zero", "separate output with NUL rather than newline"),
        getopts::optflag("", "help", "display this help and exit"),
        getopts::optflag("", "version", "output version information and exit"),
    ];

    let matches = match getopts::getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(f) => fail!("Invalid options\n{}", f.to_err_msg())
    };

    if matches.opt_present("help") {
        println!("dirname {:s} - strip last component from file name", VERSION);
        println!("");
        println!("Usage:");
        println!("  {0:s} [OPTION] NAME...", program);
        println!("");
        print(getopts::usage("Output each NAME with its last non-slash component and trailing slashes
removed; if NAME contains no  /'s,  output  '.'  (meaning  the  current
directory).", opts).as_slice());
        return 0;
    }

    if matches.opt_present("version") {
        println!("dirname version: {:s}", VERSION);
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
            if d.is_some() {
                print(d.unwrap());
            }
            print(separator);
        }
    } else {
        println!("{0:s}: missing operand", program);
        println!("Try '{0:s} --help' for more information.", program);
    }

    0
}
