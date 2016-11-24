#![crate_name = "uu_sum"]

/*
* This file is part of the uutils coreutils package.
*
* (c) T. Jameson Little <t.jameson.little@gmail.com>
*
* For the full copyright and license information, please view the LICENSE file
* that was distributed with this source code.
*/

extern crate getopts;

#[macro_use]
extern crate uucore;

use std::fs::File;
use std::io::{Read, Result, stdin, Write};
use std::path::Path;

static NAME: &'static str = "sum";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

fn bsd_sum(mut reader: Box<Read>) -> (usize, u16) {
    let mut buf = [0; 1024];
    let mut blocks_read = 0;
    let mut checksum: u16 = 0;
    loop {
        match reader.read(&mut buf) {
            Ok(n) if n != 0 => {
                blocks_read += 1;
                for &byte in buf[..n].iter() {
                    checksum = (checksum >> 1) + ((checksum & 1) << 15);
                    checksum = checksum.wrapping_add(byte as u16);
                }
            },
            _ => break,
        }
    }

    (blocks_read, checksum)
}

fn sysv_sum(mut reader: Box<Read>) -> (usize, u16) {
    let mut buf = [0; 512];
    let mut blocks_read = 0;
    let mut ret = 0u32;

    loop {
        match reader.read(&mut buf) {
            Ok(n) if n != 0 => {
                blocks_read += 1;
                for &byte in buf[..n].iter() {
                    ret = ret.wrapping_add(byte as u32);
                }
            },
            _ => break,
        }
    }

    ret = (ret & 0xffff) + (ret >> 16);
    ret = (ret & 0xffff) + (ret >> 16);

    (blocks_read, ret as u16)
}

fn open(name: &str) -> Result<Box<Read>> {
    match name {
        "-" => Ok(Box::new(stdin()) as Box<Read>),
        _ => {
            let f = try!(File::open(&Path::new(name)));
            Ok(Box::new(f) as Box<Read>)
        }
    }
}

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = getopts::Options::new();

    opts.optflag("r", "", "use the BSD compatible algorithm (default)");
    opts.optflag("s", "sysv", "use System V compatible algorithm");
    opts.optflag("h", "help", "show this help message");
    opts.optflag("v", "version", "print the version and exit");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => crash!(1, "Invalid options\n{}", f)
    };

    if matches.opt_present("help") {
        let msg = format!("{0} {1}

Usage:
  {0} [OPTION]... [FILE]...

Checksum and count the blocks in a file.", NAME, VERSION);
        println!("{}\nWith no FILE, or when  FILE is -, read standard input.", opts.usage(&msg));
        return 0;
    }
    if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
        return 0;
    }

    let sysv = matches.opt_present("sysv");

    let files = if matches.free.is_empty() {
        vec!["-".to_owned()]
    } else {
        matches.free
    };

    let print_names = if sysv {
        files.len() > 1 || files[0] != "-"
    } else {
        files.len() > 1
    };

    for file in &files {
        let reader = match open(file) {
            Ok(f) => f,
            _ => crash!(1, "unable to open file")
        };
        let (blocks, sum) = if sysv {
            sysv_sum(reader)
        } else {
            bsd_sum(reader)
        };

        if print_names {
            println!("{} {} {}", sum, blocks, file);
        } else {
            println!("{} {}", sum, blocks);
        }
    }

    0
}
