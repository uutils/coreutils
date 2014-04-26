#![crate_id(name="uname", version="1.0.0", author="joaoxsouls")]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Joao Oliveira <joaoxsouls@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

/* last synced with: uname (GNU coreutils) 8.21 */

#![allow(non_camel_case_types)]
#![feature(macro_rules)]

extern crate getopts;
extern crate libc;

use std::os;
use std::mem::uninit;
use std::io::print;
use std::str::raw::from_c_str;
use c_types::utsname;

#[path = "../common/util.rs"] mod util;
#[path = "../common/c_types.rs"] mod c_types;

struct utsrust {
    sysname: ~str,
    nodename: ~str,
    release: ~str,
    version: ~str,
    machine: ~str
}

extern {
    fn uname(uts: *mut utsname);
}

unsafe fn getuname() -> utsrust {
    let mut uts: utsname = uninit();
    uname(&mut uts);
    utsrust {
        sysname: from_c_str(uts.sysname.as_ptr()), nodename: from_c_str(uts.nodename.as_ptr()),
        release: from_c_str(uts.release.as_ptr()), version: from_c_str(uts.version.as_ptr()),
        machine: from_c_str(uts.machine.as_ptr())
    }
}


static NAME: &'static str = "uname";

fn main() {
    let args = os::args();
    let program = args[0].as_slice();
    let opts = ~[
        getopts::optflag("h", "help", "display this help and exit"),
        getopts::optflag("a", "all", "Behave as though all of the options -mnrsv were specified."),
        getopts::optflag("m", "machine", "print the machine hardware name."),
        getopts::optflag("n", "nodename", "print the nodename (the nodename may be a name that the system is known by to a communications network)."),
        getopts::optflag("p", "processor", "print the machine processor architecture name."),
        getopts::optflag("r", "release", "print the operating system release."),
        getopts::optflag("s", "sysname", "print the operating system name."),
        getopts::optflag("v", "version", "print the operating system version."),
    ];
    let matches = match getopts::getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(f) => crash!(1, "{}", f.to_err_msg()),
    };
    if matches.opt_present("help") {
        println!("uname 1.0.0");
        println!("");
        println!("Usage:");
        println!("  {:s}", program);
        println!("");
        print(getopts::usage("The uname utility writes symbols representing one or more system characteristics to the standard output.", opts));
        return;
    }
    let uname = unsafe { getuname() };
    let mut output = StrBuf::new();
    if matches.opt_present("sysname") || matches.opt_present("all")
        || !matches.opts_present([~"nodename", ~"release", ~"version", ~"machine"]) {
            output.push_str(uname.sysname);
            output.push_str(" ");
    }

    if matches.opt_present("nodename") || matches.opt_present("all") {
        output.push_str(uname.nodename);
        output.push_str(" ");
    }
    if matches.opt_present("release") || matches.opt_present("all") {
        output.push_str(uname.release);
        output.push_str(" ");
    }
    if matches.opt_present("version") || matches.opt_present("all") {
        output.push_str(uname.version);
        output.push_str(" ");
    }
    if matches.opt_present("machine") || matches.opt_present("all") {
        output.push_str(uname.machine);
        output.push_str(" ");
    }
    println!("{}", output.as_slice().trim_left())
}
