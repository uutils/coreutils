#![crate_name = "uu_hostid"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Maciej Dziardziel <fiedzia@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE file
 * that was distributed with this source code.
 */

extern crate libc;

#[macro_use]
extern crate uucore;

use libc::c_long;

static SYNTAX: &'static str = "[options]"; 
static SUMMARY: &'static str = ""; 
static LONG_HELP: &'static str = ""; 

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
    new_coreopts!(SYNTAX, SUMMARY, LONG_HELP)
        .parse(args);
    hostid();
    0
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
