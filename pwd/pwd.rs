#[crate_id(name="pwd", vers="1.0.0", author="Heather Cynede")];

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Derek Chiang <derekchiang93@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern mod extra;

use std::os;
use std::io::{print, stderr};
use extra::getopts::groups;

static VERSION: &'static str = "1.0.0";

fn main() {
    let args = os::args();
    let program = args[0].clone();
    let opts = ~[
        groups::optflag("", "help", "display this help and exit"),
        groups::optflag("", "version", "output version information and exit"),
    ];

    let matches = match groups::getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(f) => {
            writeln!(&mut stderr() as &mut Writer,
                "Invalid options\n{}", f.to_err_msg());
            os::set_exit_status(1);
            return
        }  
    };

    if matches.opt_present("help") {
        println!("pwd {}", VERSION);
        println!("");
        println!("Usage:");
        println!("  {0:s} [OPTION] NAME...", program);
        println!("");
        print(groups::usage("Print the full filename of the current working directory.", opts));
    } else if matches.opt_present("version") {
        return println!("pwd version: {}", VERSION);
    } else {
        let cwd = std::os::getcwd();
        println!("{}", cwd.display());
    }
}
