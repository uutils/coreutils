#![crate_name = "uu_uname"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Joao Oliveira <joaoxsouls@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

/* last synced with: uname (GNU coreutils) 8.21 */

extern crate getopts;
extern crate libc;

#[macro_use]
extern crate uucore;

use std::ffi::CStr;
use std::io::Write;
use std::mem::uninitialized;
use uucore::c_types::utsname;

struct Uts {
    sysname: String,
    nodename: String,
    release: String,
    version: String,
    machine: String
}

extern {
    fn uname(uts: *mut utsname);
}

unsafe fn string_from_c_str(ptr: *const i8) -> String {
    String::from_utf8_lossy(CStr::from_ptr(ptr as *const std::os::raw::c_char).to_bytes()).to_string()
}

unsafe fn getuname() -> Uts {
    let mut uts: utsname = uninitialized();
    uname(&mut uts);
    Uts {
        sysname:  string_from_c_str(uts.sysname.as_ptr()  as *const i8), 
        nodename: string_from_c_str(uts.nodename.as_ptr() as *const i8),
        release:  string_from_c_str(uts.release.as_ptr()  as *const i8), 
        version:  string_from_c_str(uts.version.as_ptr()  as *const i8),
        machine:  string_from_c_str(uts.machine.as_ptr()  as *const i8)
    }
}

static NAME: &'static str = "uname";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = getopts::Options::new();

    opts.optflag("h", "help", "display this help and exit");
    opts.optflag("a", "all", "Behave as though all of the options -mnrsv were specified.");
    opts.optflag("m", "machine", "print the machine hardware name.");
    opts.optflag("n", "nodename", "print the nodename (the nodename may be a name that the system is known by to a communications network).");
    opts.optflag("p", "processor", "print the machine processor architecture name.");
    opts.optflag("r", "release", "print the operating system release.");
    opts.optflag("s", "sysname", "print the operating system name.");
    opts.optflag("v", "version", "print the operating system version.");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => crash!(1, "{}", f),
    };
    if matches.opt_present("help") {
        println!("{} {}", NAME, VERSION);
        println!("");
        println!("Usage:");
        println!("  {} [OPTIONS]", NAME);
        println!("");
        print!("{}", opts.usage("The uname utility writes symbols representing one or more system characteristics to the standard output."));
        return 0;
    }
    let uname = unsafe { getuname() };
    let mut output = String::new();
    if matches.opt_present("sysname") || matches.opt_present("all")
        || !matches.opts_present(&["nodename".to_owned(), "release".to_owned(), "version".to_owned(), "machine".to_owned()]) {
            output.push_str(uname.sysname.as_ref());
            output.push_str(" ");
    }

    if matches.opt_present("nodename") || matches.opt_present("all") {
        output.push_str(uname.nodename.as_ref());
        output.push_str(" ");
    }
    if matches.opt_present("release") || matches.opt_present("all") {
        output.push_str(uname.release.as_ref());
        output.push_str(" ");
    }
    if matches.opt_present("version") || matches.opt_present("all") {
        output.push_str(uname.version.as_ref());
        output.push_str(" ");
    }
    if matches.opt_present("machine") || matches.opt_present("all") {
        output.push_str(uname.machine.as_ref());
        output.push_str(" ");
    }
    println!("{}", output.trim());

    0
}
