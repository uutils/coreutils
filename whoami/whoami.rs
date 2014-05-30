#![crate_id(name="whoami", version="1.0.0", author="KokaKiwi")]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Jordi Boggiano <j.boggiano@seld.be>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

/* last synced with: whoami (GNU coreutils) 8.21 */

#![allow(non_camel_case_types)]

#![feature(macro_rules)]

extern crate getopts;
extern crate libc;

use std::io::print;
use std::os;
use std::str;
use c_types::{c_passwd, getpwuid};

#[path = "../common/util.rs"] mod util;
#[path = "../common/c_types.rs"] mod c_types;

extern {
    pub fn geteuid() -> libc::c_int;
}

unsafe fn getusername() -> String {
    let passwd: *c_passwd = getpwuid(geteuid());

    let pw_name: *libc::c_char = (*passwd).pw_name;
    let name = str::raw::from_c_str(pw_name);

    name
}

static NAME: &'static str = "whoami";

#[allow(dead_code)]
fn main() { uumain(os::args()); }

pub fn uumain(args: Vec<String>) {
    let program = args.get(0).as_slice();
    let opts = [
        getopts::optflag("h", "help", "display this help and exit"),
        getopts::optflag("V", "version", "output version information and exit"),
    ];
    let matches = match getopts::getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(f) => crash!(1, "{}", f.to_err_msg()),
    };
    if matches.opt_present("help") {
        println!("whoami 1.0.0");
        println!("");
        println!("Usage:");
        println!("  {:s}", program);
        println!("");
        print(getopts::usage("print effective userid", opts).as_slice());
        return;
    }
    if matches.opt_present("version") {
        println!("whoami 1.0.0");
        return;
    }

    exec();
}

pub fn exec() {
    unsafe {
        let username = getusername();
        println!("{:s}", username);
    }
}
