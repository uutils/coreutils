#![crate_id(name="sync", vers="1.0.0", author="Alexander Fomin")]
/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Alexander Fomin <xander.fomin@ya.ru>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

 /* Last synced with: sync (GNU coreutils) 8.13 */

extern crate getopts;
extern crate libc;

use std::os;
use getopts::{optflag, getopts, usage};

extern {
    fn sync() -> libc::c_void;
}

#[allow(dead_code)]
fn main () { os::set_exit_status(uumain(os::args())); }

pub fn uumain(args: Vec<String>) -> int {
    let program = args.get(0);

    let options = [
        optflag("h", "help", "display this help and exit"),
        optflag("V", "version", "output version information and exit")
    ];

    let matches = match getopts(args.tail(), options) {
        Ok(m) => { m }
        _ => { help(program.as_slice(), options); return 0 }
    };

    if matches.opt_present("h") {
        help(program.as_slice(), options);
        return 0
    }

    if matches.opt_present("V") {
        version();
        return 0
    }

    unsafe {
        sync()
    };

    0
}

fn version() {
    println!("sync (uutils) 1.0.0");
    println!("The MIT License");
    println!("");
    println!("Author -- Alexander Fomin.");
}

fn help(program: &str, options: &[getopts::OptGroup]) {
    println!("Usage: {:s} [OPTION]", program);
    print!("{:s}", usage("Force changed blocks to disk, update the super block.", options));
}
