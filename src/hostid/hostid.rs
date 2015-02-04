#![crate_name = "hostid"]
#![feature(collections, core, libc, rustc_private)]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Maciej Dziardziel <fiedzia@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE file
 * that was distributed with this source code.
 */


extern crate getopts;
extern crate collections;
extern crate serialize;
extern crate libc;


#[macro_use] extern crate log;

use getopts::{
    getopts,
    optflag,
    usage,
};

use libc::{c_long};

#[path = "../common/util.rs"]
#[macro_use]
mod util;

static NAME:     &'static str = "hostid";
static VERSION:  &'static str = "0.0.1";

static EXIT_ERR: isize = 1;

pub enum Mode {
    HostId,
    Help,
    Version,
}

impl Copy for Mode {}

//currently rust libc interface doesn't include gethostid
extern {
    pub fn gethostid() -> c_long;
}

pub fn uumain(args: Vec<String>) -> isize {

    let opts = [
        optflag("", "help", "display this help and exit"),
        optflag("", "version", "output version information and exit"),
    ];

    let usage = usage("[options]", &opts);


    let matches = match getopts(args.tail(), &opts) {
        Ok(m) => m,
        Err(e) => {
            show_error!("{}\n{}", e,  get_help_text(NAME, usage.as_slice()));
            return EXIT_ERR;
        },
    };

    let mode = if matches.opt_present("version") {
        Mode::Version
    } else if matches.opt_present("help") {
        Mode::Help
    } else {
        Mode::HostId
    };

    match mode {
        Mode::HostId  => hostid(),
        Mode::Help    => help(NAME, usage.as_slice()),
        Mode::Version => version(),
    }

    0
}

fn version() {
    println!("{} {}", NAME, VERSION);
}

fn get_help_text(progname: &str, usage: &str) -> String {
    format!("Usage: \n {0} {1}", progname, usage)
}

fn help(progname: &str, usage: &str) {
    println!("{}", get_help_text(progname, usage));
}

fn hostid() {

  /* POSIX says gethostid returns a "32-bit identifier" but is silent
     whether it's sign-extended.  Turn off any sign-extension.  This
     is a no-op unless unsigned int is wider than 32 bits.  */

    let mut result:c_long;
    unsafe {
        result = gethostid();
    }
    
    result &= 0xffffffff; 
    println!("{:0>8x}", result);
}
