#![crate_name = "uu_unlink"]

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

#[macro_use]
extern crate uucore;

use getopts::Options;
use libc::{S_IFLNK, S_IFMT, S_IFREG};
use libc::{lstat, stat, unlink};
use std::io::{Error, ErrorKind};
use std::mem::uninitialized;
use std::ffi::CString;

static NAME: &str = "unlink";
static VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = Options::new();

    opts.optflag("h", "help", "display this help and exit");
    opts.optflag("V", "version", "output version information and exit");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => crash!(1, "invalid options\n{}", f),
    };

    if matches.opt_present("help") {
        println!("{} {}", NAME, VERSION);
        println!();
        println!("Usage:");
        println!("  {} [FILE]... [OPTION]...", NAME);
        println!();
        println!("{}", opts.usage("Unlink the file at [FILE]."));
        return 0;
    }

    if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
        return 0;
    }

    if matches.free.is_empty() {
        crash!(
            1,
            "missing operand\nTry '{0} --help' for more information.",
            NAME
        );
    } else if matches.free.len() > 1 {
        crash!(
            1,
            "extra operand: '{1}'\nTry '{0} --help' for more information.",
            NAME,
            matches.free[1]
        );
    }

    let c_string = CString::new(matches.free[0].clone()).unwrap();
    //^ unwrap() cannot fail, the string comes from argv so it cannot contain a \0.

    let st_mode = {
        let mut buf: stat = unsafe { uninitialized() };
        let result = unsafe {
            lstat(
                c_string.as_ptr(),
                &mut buf as *mut stat,
            )
        };

        if result < 0 {
            crash!(
                1,
                "Cannot stat '{}': {}",
                matches.free[0],
                Error::last_os_error()
            );
        }

        buf.st_mode & S_IFMT
    };

    let result = if st_mode != S_IFREG && st_mode != S_IFLNK {
        Err(Error::new(
            ErrorKind::Other,
            "Not a regular file or symlink",
        ))
    } else {
        let result = unsafe { unlink(c_string.as_ptr()) };

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
