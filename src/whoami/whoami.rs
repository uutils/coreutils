#![crate_name = "whoami"]
#![feature(collections, core, io, libc, rustc_private, std_misc)]

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

extern crate getopts;
extern crate libc;

use std::old_io::print;

#[path = "../common/util.rs"] #[macro_use] mod util;

#[cfg(unix)]
mod platform {
    use super::libc;
    use self::c_types::{c_passwd, getpwuid};

    #[path = "../../common/c_types.rs"] mod c_types;

    extern {
        pub fn geteuid() -> libc::uid_t;
    }

    pub unsafe fn getusername() -> String {
        let passwd: *const c_passwd = getpwuid(geteuid());

        let pw_name: *const libc::c_char = (*passwd).pw_name;
        String::from_utf8_lossy(::std::ffi::c_str_to_bytes(&pw_name)).to_string()
    }
}

#[cfg(windows)]
mod platform {
    pub use super::libc;
    use std::mem;

    extern "system" {
        pub fn GetUserNameA(out: *mut libc::c_char, len: *mut libc::uint32_t) -> libc::uint8_t;
    }

    #[allow(unused_unsafe)]
    pub unsafe fn getusername() -> String {
        let mut buffer: [libc::c_char; 2048] = mem::uninitialized();   // XXX: it may be possible that this isn't long enough.  I don't know
        if !GetUserNameA(buffer.as_mut_ptr(), &mut (buffer.len() as libc::uint32_t)) == 0 {
            crash!(1, "username is too long");
        }
        String::from_utf8_lossy(::std::ffi::c_str_to_bytes(&buffer.as_ptr())).to_string()
    }
}

static NAME: &'static str = "whoami";

pub fn uumain(args: Vec<String>) -> isize {
    let program = args[0].as_slice();
    let opts = [
        getopts::optflag("h", "help", "display this help and exit"),
        getopts::optflag("V", "version", "output version information and exit"),
    ];
    let matches = match getopts::getopts(args.tail(), &opts) {
        Ok(m) => m,
        Err(f) => crash!(1, "{}", f),
    };
    if matches.opt_present("help") {
        println!("whoami 1.0.0");
        println!("");
        println!("Usage:");
        println!("  {}", program);
        println!("");
        print(getopts::usage("print effective userid", &opts).as_slice());
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
        println!("{}", username);
    }
}
