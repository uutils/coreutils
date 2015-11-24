#![crate_name = "hostid"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Maciej Dziardziel <fiedzia@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE file
 * that was distributed with this source code.
 */

extern crate getopts;
extern crate libc;

use libc::c_long;

#[path = "../common/util.rs"]
#[macro_use]
mod util;

static NAME: &'static str = "hostid";
static VERSION: &'static str = "0.0.1";

static EXIT_ERR: i32 = 1;

pub enum Mode {
    HostId,
    Help,
    Version,
}

// currently rust libc interface doesn't include gethostid
extern {
    pub fn gethostid() -> c_long;
}

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = getopts::Options::new();
    opts.optflag("", "help", "display this help and exit");
    opts.optflag("", "version", "output version information and exit");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(_) => {
            help(&opts);
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
        Mode::Help    => help(&opts),
        Mode::Version => version(),
    }

    0
}

fn version() {
    println!("{} {}", NAME, VERSION);
}

fn help(opts: &getopts::Options) {
    let msg = format!("Usage:\n {} [options]", NAME);
    print!("{}", opts.usage(&msg));
}

fn hostid() {
  /*
   * POSIX says gethostid returns a "32-bit identifier" but is silent
   * whether it's sign-extended.  Turn off any sign-extension.  This
   * is a no-op unless unsigned int is wider than 32 bits.
   */

    let mut result:c_long;
    unsafe {
        result = gethostid();
    }

    result &= 0xffffffff; 
    println!("{:0>8x}", result);
}

#[allow(dead_code)]
fn main() {
    std::process::exit(uumain(std::env::args().collect()));
}
