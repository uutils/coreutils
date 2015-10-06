#![crate_name = "cksum"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Michael Gehring <mg@ebfe.org>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate getopts;

use getopts::Options;
use std::fs::File;
use std::io::{self, stdin, Read, Write, BufReader};
use std::mem;
use std::path::Path;

use crc_table::CRC_TABLE;

#[path="../common/util.rs"]
#[macro_use]
mod util;

mod crc_table;

static NAME: &'static str = "cksum";
static VERSION: &'static str = "1.0.0";

#[inline]
fn crc_update(crc: u32, input: u8) -> u32 {
    (crc << 8) ^ CRC_TABLE[((crc >> 24) as usize ^ input as usize) & 0xFF]
}

#[inline]
fn crc_final(mut crc: u32, mut length: usize) -> u32 {
    while length != 0 {
        crc = crc_update(crc, length as u8);
        length >>= 8;
    }

    !crc
}

#[inline]
fn cksum(fname: &str) -> io::Result<(u32, usize)> {
    let mut crc = 0u32;
    let mut size = 0usize;

    let file;
    let mut rd: Box<Read> = match fname {
        "-" => {
            Box::new(stdin())
        }
        _ => {
            file = try!(File::open(&Path::new(fname)));
            Box::new(BufReader::new(file))
        }
    };

    let mut bytes: [u8; 1024 * 1024] = unsafe { mem::uninitialized() };
    loop {
        match rd.read(&mut bytes) {
            Ok(num_bytes) => {
                if num_bytes == 0 {
                    return Ok((crc_final(crc, size), size));
                }
                for &b in bytes[..num_bytes].iter() {
                    crc = crc_update(crc, b);
                }
                size += num_bytes;
            }
            Err(err) => return Err(err),
        }
    }
}

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = Options::new();
    opts.optflag("h", "help", "display this help and exit");
    opts.optflag("V", "version", "output version information and exit");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(err) => panic!("{}", err),
    };

    if matches.opt_present("help") {
        let msg = format!("{0} {1}

Usage:
  {0} [OPTIONS] [FILE]...

Print CRC and size for each file.",
                          NAME,
                          VERSION);

        print!("{}", opts.usage(&msg));
        return 0;
    }

    if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
        return 0;
    }

    let files = matches.free;

    if files.is_empty() {
        match cksum("-") {
            Ok((crc, size)) => println!("{} {}", crc, size),
            Err(err) => {
                show_error!("{}", err);
                return 2;
            }
        }
        return 0;
    }

    let mut exit_code = 0;
    for fname in files.iter() {
        match cksum(fname.as_ref()) {
            Ok((crc, size)) => println!("{} {} {}", crc, size, fname),
            Err(err) => {
                show_error!("'{}' {}", fname, err);
                exit_code = 2;
            }
        }
    }

    exit_code
}
