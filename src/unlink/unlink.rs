#![crate_name = "unlink"]
#![feature(rustc_private)]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Colin Warren <me@zv.ms>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

/* last synced with: unlink (GNU coreutils) 8.21 */

extern crate getopts;
extern crate libc;

use libc::consts::os::posix88::{S_IFMT, S_IFLNK, S_IFREG};
use libc::types::os::arch::c95::c_char;
use libc::types::os::arch::posix01::stat;
use libc::funcs::posix01::stat_::lstat;
use libc::funcs::posix88::unistd::unlink;

use std::mem::uninitialized;
use std::io::{Error, ErrorKind, Write};

#[path = "../common/util.rs"]
#[macro_use]
mod util;

static NAME: &'static str = "unlink";

pub fn uumain(args: Vec<String>) -> i32 {
    let opts = [
        getopts::optflag("h", "help", "display this help and exit"),
        getopts::optflag("V", "version", "output version information and exit"),
    ];

    let matches = match getopts::getopts(&args[1..], &opts) {
        Ok(m) => m,
        Err(f) => {
            crash!(1, "invalid options\n{}", f)
        }
    };

    if matches.opt_present("help") {
        println!("unlink 1.0.0");
        println!("");
        println!("Usage:");
        println!("  {0} [FILE]... [OPTION]...", args[0]);
        println!("");
        println!("{}", getopts::usage("Unlink the file at [FILE].", &opts));
        return 0;
    }

    if matches.opt_present("version") {
        println!("unlink 1.0.0");
        return 0;
    }

    if matches.free.len() == 0 {
        crash!(1, "missing operand\nTry '{0} --help' for more information.", args[0]);
    } else if matches.free.len() > 1 {
        crash!(1, "extra operand: '{1}'\nTry '{0} --help' for more information.", args[0], matches.free[1]);
    }

    let st_mode = {
        let mut buf: stat = unsafe { uninitialized() };
        let result = unsafe { lstat(matches.free[0].as_ptr() as *const c_char, &mut buf as *mut stat) };

        if result < 0 {
            crash!(1, "Cannot stat '{}': {}", matches.free[0], Error::last_os_error());
        }

        buf.st_mode & S_IFMT
    };

    let result = if st_mode != S_IFREG && st_mode != S_IFLNK {
        Err(Error::new(ErrorKind::Other, "Not a regular file or symlink"))
    } else {
        let result = unsafe { unlink(matches.free[0].as_ptr() as *const c_char) };

        if result < 0 {
            Err(Error::last_os_error())
        } else {
            Ok(())
        }
    };

    match result {
        Ok(_) => (),
        Err(e) => {
            crash!(1, "cannot unlink '{0}': {1}", matches.free[0], e);
        }
    }

    0
}
