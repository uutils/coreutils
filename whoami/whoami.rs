#![crate_name = "whoami"]

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

#[path = "../common/util.rs"] mod util;

#[cfg(unix)]
mod platform {
    use super::libc;
    use std::str;
    use self::c_types::{c_passwd, getpwuid};

    #[path = "../../common/c_types.rs"] mod c_types;

    extern {
        pub fn geteuid() -> libc::uid_t;
    }

    pub unsafe fn getusername() -> String {
        let passwd: *const c_passwd = getpwuid(geteuid());

        let pw_name: *const libc::c_char = (*passwd).pw_name;
        let name = str::raw::from_c_str(pw_name);

        name
    }
}

#[cfg(windows)]
mod platform {
    pub use super::libc;
    use std::mem;
    use std::str;

    extern "system" {
        pub fn GetUserNameA(out: *mut libc::c_char, len: *mut libc::uint32_t) -> libc::uint8_t;
    }

    #[allow(unused_unsafe)]
    pub unsafe fn getusername() -> String {
        let buffer: [libc::c_char, ..2048] = mem::uninitialized();   // XXX: it may be possible that this isn't long enough.  I don't know
        if !GetUserNameA(buffer.as_ptr(), &(buffer.len() as libc::uint32_t)) == 0 {
            crash!(1, "username is too long");
        }
        str::raw::from_c_str(buffer.as_ptr())
    }
}

static NAME: &'static str = "whoami";

pub fn uumain(args: Vec<String>) -> int {
    let program = args[0].as_slice();
    let opts = [
        getopts::optflag("h", "help", "display this help and exit"),
        getopts::optflag("V", "version", "output version information and exit"),
    ];
    let matches = match getopts::getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(f) => crash!(1, "{}", f),
    };
    if matches.opt_present("help") {
        println!("whoami 1.0.0");
        println!("");
        println!("Usage:");
        println!("  {:s}", program);
        println!("");
        print(getopts::usage("print effective userid", opts).as_slice());
        return 0;
    }
    if matches.opt_present("version") {
        println!("whoami 1.0.0");
        return 0;
    }

    exec();

    0
}

pub fn exec() {
    unsafe {
        let username = platform::getusername();
        println!("{:s}", username);
    }
}
