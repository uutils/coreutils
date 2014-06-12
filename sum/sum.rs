#![crate_id(name="sum", vers="1.0.0", author="T. Jameson Little")]
#![feature(macro_rules)]

/*
* This file is part of the uutils coreutils package.
*
* (c) T. Jameson Little <t.jameson.little@gmail.com>
*
* For the full copyright and license information, please view the LICENSE file
* that was distributed with this source code.
*/

extern crate getopts;
extern crate libc;

use std::os;
use std::io::{File, IoResult, print};
use std::io::stdio::{stdin_raw};

#[path="../common/util.rs"]
mod util;

static VERSION: &'static str = "1.0.0";
static NAME: &'static str = "sum";

fn bsd_sum(mut reader: Box<Reader>) -> (uint, u16) {
    let mut buf = [0, .. 1024];
    let mut blocks_read = 0;
    let mut checksum: u16 = 0;
    loop {
        match reader.read(buf) {
            Ok(n) if n != 0 => {
                blocks_read += 1;
                for &byte in buf.slice_to(n).iter() {
                    checksum = (checksum >> 1) + ((checksum & 1) << 15);
                    checksum += byte as u16;
                }
            },
            _ => break,
        }
    }

    (blocks_read, checksum)
}

fn sysv_sum(mut reader: Box<Reader>) -> (uint, u16) {
    let mut buf = [0, .. 512];
    let mut blocks_read = 0;
    let mut ret = 0;

    loop {
        match reader.read(buf) {
            Ok(n) if n != 0 => {
                blocks_read += 1;
                for &byte in buf.slice_to(n).iter() {
                    ret += byte as u32;
                }
            },
            _ => break,
        }
    }

    ret = (ret & 0xffff) + (ret >> 16);
    ret = (ret & 0xffff) + (ret >> 16);

    (blocks_read, ret as u16)
}

fn open(name: &str) -> IoResult<Box<Reader>> {
	match name {
		"-" => Ok(box stdin_raw() as Box<Reader>),
		_ => {
			let f = try!(File::open(&Path::new(name)));
			Ok(box f as Box<Reader>)
		}
	}
}

#[allow(dead_code)]
fn main() { os::set_exit_status(uumain(os::args())); }

pub fn uumain(args: Vec<String>) -> int {
    let program = args.get(0).as_slice();
    let opts = [
        getopts::optflag("r", "", "use the BSD compatible algorithm (default)"),
        getopts::optflag("s", "sysv", "use System V compatible algorithm"),
        getopts::optflag("h", "help", "show this help message"),
        getopts::optflag("v", "version", "print the version and exit"),
    ];

    let matches = match getopts::getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(f) => crash!(1, "Invalid options\n{}", f.to_err_msg())
    };

    if matches.opt_present("help") {
        println!("{} {}", program, VERSION);
        println!("");
        println!("Usage:");
        println!("  {0:s} [OPTION]... [FILE]...", program);
        println!("");
        print(getopts::usage("checksum and count the blocks in a file", opts).as_slice());
        println!("");
        println!("With no FILE, or when  FILE is -, read standard input.");
        return 0;
    }
    if matches.opt_present("version") {
        println!("{} {}", program, VERSION);
        return 0;
    }

    let sysv = matches.opt_present("sysv");

    let file = if matches.free.is_empty() {
        "-"
    } else {
        matches.free.get(0).as_slice()
    };

    let reader = match open(file) {
        Ok(f) => f,
        _ => crash!(1, "unable to open file")
    };
    let (blocks, sum) = if sysv {
        sysv_sum(reader)
    } else {
        bsd_sum(reader)
    };

    println!("{} {}", sum, blocks);

    0
}
