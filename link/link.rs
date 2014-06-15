#![crate_id(name="link", vers="1.0.0", author="Morten Olsen Lysgaard")]
/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Morten Olsen Lysgaard <morten@lysgaard.no>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 *
 */

extern crate getopts;

use std::os;
use std::io::fs::link;
use std::path::Path;
use getopts::{optflag, getopts, usage};

static PROGRAM: &'static str = "link";

#[allow(dead_code)]
fn main () { os::set_exit_status(uumain(os::args())); }

pub fn uumain(args: Vec<String>) -> int {

    let possible_options = [
        optflag("h", "help", "help"),
        optflag("V", "version", "version"),
    ];

    let given_options = match getopts(args.as_slice(), possible_options) {
        Ok (m) => { m }
        Err(_) => {
            println!("{:s}", usage(PROGRAM, possible_options));
            return 0;
        }
    };

    if given_options.opt_present("h") {
        println!("{:s}", usage(PROGRAM, possible_options));
        return 0;
    }
    if given_options.opt_present("V") { version(); return 0; }

    let files = given_options.free;

    if files.len() != 3 {
        println!("{:s}", usage(PROGRAM, possible_options));
        return 0;
    } else {
        let src = Path::new(files.get(1).as_slice());
        let dst = Path::new(files.get(2).as_slice());
        match link(&src, &dst) {
          Ok(()) => return 1,
          Err(err) => {println!("{}", err.to_str()); return 1;}
        }
    }
}

fn version () {
    println!("link version 1.0.0");
}
