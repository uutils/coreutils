#![crate_id(name="hostid", vers="0.0.1", author="Maciej Dziardziel")]
#![feature(macro_rules)]
#![feature(phase)]

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


#[phase(syntax, link)] extern crate log;

use std::os;

use getopts::{
    getopts,
    optflag,
    usage,
};

use libc::{c_long};

#[path = "../common/util.rs"]
mod util;

static NAME:     &'static str = "hostid";
static VERSION:  &'static str = "0.0.1";

static EXIT_ERR: i32 = 1;

pub enum Mode {
    HostId,
    Help,
    Version,
}

//currently rust libc interface doesn't include gethostid
extern {
    pub fn gethostid() -> c_long;
}

fn main() {
    let args: Vec<StrBuf> = os::args().iter().map(|x| x.to_strbuf()).collect();

    let opts = ~[
        optflag("", "help", "display this help and exit"),
        optflag("", "version", "output version information and exit"),
    ];

    let usage = usage("[options]", opts);


    let matches = match getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(e) => {
            show_error!(EXIT_ERR, "{}\n{}", e.to_err_msg(),  get_help_text(NAME, usage.as_slice()));
            return
        },
    };

    let mode = if matches.opt_present("version") {
        Version
    } else if matches.opt_present("help") {
        Help
    } else {
        HostId
    };

    match mode {
        HostId  => hostid(),
        Help    => help(NAME, usage.as_slice()),
        Version => version(),
    }
}

fn version() {
    println!("{} {}", NAME, VERSION);
}

fn get_help_text(progname: &str, usage: &str) -> ~str {
    let msg = format!("Usage: \n {0} {1}", progname, usage);
    msg
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
