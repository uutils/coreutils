#[crate_id(name="pwd", vers="1.0.0", author="Heather Cynede")];

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Derek Chiang <derekchiang93@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

#[feature(macro_rules)];

extern crate extra;
extern crate getopts;

use std::os;
use std::io::print;

#[path = "../util.rs"]
mod util;

static NAME: &'static str = "pwd";
static VERSION: &'static str = "1.0.0";

fn main() {
    let args = os::args();
    let program = args[0].clone();
    let opts = ~[
        getopts::optflag("", "help", "display this help and exit"),
        getopts::optflag("", "version", "output version information and exit"),
    ];

    let matches = match getopts::getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(f) => {
            crash!(1, "Invalid options\n{}", f.to_err_msg())
        }
    };

    if matches.opt_present("help") {
        println!("pwd {}", VERSION);
        println!("");
        println!("Usage:");
        println!("  {0:s} [OPTION] NAME...", program);
        println!("");
        print(getopts::usage("Print the full filename of the current working directory.", opts));
    } else if matches.opt_present("version") {
        return println!("pwd version: {}", VERSION);
    } else {
        let cwd = std::os::getcwd();
        println!("{}", cwd.display());
    }
}
