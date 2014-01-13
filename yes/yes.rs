#[crate_id(name="yes", vers="1.0.0", author="Seldaek")];

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Jordi Boggiano <j.boggiano@seld.be>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

/* last synced with: yes (GNU coreutils) 8.13 */

extern mod extra;

use std::os;
use std::io::{print, println, stderr};
use extra::getopts::groups;

fn main() {
    let args = os::args();
    let program = args[0].clone();
    let opts = ~[
        groups::optflag("h", "help", "display this help and exit"),
        groups::optflag("V", "version", "output version information and exit"),
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
        println!("yes 1.0.0");
        println!("");
        println!("Usage:");
        println!("  {0:s} [STRING]... [OPTION]...", program);
        println!("");
        print(groups::usage("Repeatedly output a line with all specified STRING(s), or 'y'.", opts));
        return;
    }
    if matches.opt_present("version") {
        println!("yes 1.0.0");
        return;
    }
    let mut string = ~"y";
    if !matches.free.is_empty() {
        string = matches.free.connect(" ");
    }

    exec(string);
}

pub fn exec(string: ~str) {
    loop {
        println(string);
    }
}
