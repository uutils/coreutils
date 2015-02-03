#![crate_name = "unlink"]
#![feature(collections, core, io, libc, path, rustc_private)]

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

use std::old_io as io;
use std::old_io::fs::{self, PathExtensions};
use std::old_io::print;

#[path = "../common/util.rs"]
#[macro_use]
mod util;

static NAME: &'static str = "unlink";

pub fn uumain(args: Vec<String>) -> isize {
    let program = args[0].clone();
    let opts = [
        getopts::optflag("h", "help", "display this help and exit"),
        getopts::optflag("V", "version", "output version information and exit"),
    ];

    let matches = match getopts::getopts(args.tail(), &opts) {
        Ok(m) => m,
        Err(f) => {
            crash!(1, "invalid options\n{}", f)
        }
    };

    if matches.opt_present("help") {
        println!("unlink 1.0.0");
        println!("");
        println!("Usage:");
        println!("  {0} [FILE]... [OPTION]...", program);
        println!("");
        print(getopts::usage("Unlink the file at [FILE].", &opts).as_slice());
        return 0;
    }

    if matches.opt_present("version") {
        println!("unlink 1.0.0");
        return 0;
    }

    if matches.free.len() == 0 {
        crash!(1, "missing operand\nTry '{0} --help' for more information.", program);
    } else if matches.free.len() > 1 {
        crash!(1, "extra operand: '{1}'\nTry '{0} --help' for more information.", program, matches.free[1]);
    }

    let path = Path::new(matches.free[0].clone());

    let result = path.lstat().and_then(|info| {
        match info.kind {
            io::FileType::RegularFile => Ok(()),
            io::FileType::Symlink => Ok(()),
            _ => Err(io::IoError {
                kind: io::OtherIoError,
                desc: "is not a file or symlink",
                detail: None
            })
        }
    }).and_then(|_| {
        fs::unlink(&path)
    });

    match result {
        Ok(_) => (),
        Err(e) => {
            crash!(1, "cannot unlink '{0}': {1}", path.display(), e.desc);
        }
    }

    0
}
