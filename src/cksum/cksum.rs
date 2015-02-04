#![crate_name = "cksum"]
#![feature(collections, core, io, path, rustc_private)]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Michael Gehring <mg@ebfe.org>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate getopts;

use std::old_io::{EndOfFile, File, IoError, IoResult, print};
use std::old_io::stdio::stdin_raw;
use std::mem;

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
fn cksum(fname: &str) -> IoResult<(u32, usize)> {
    let mut crc = 0u32;
    let mut size = 0us;

    let mut stdin_buf;
    let mut file_buf;
    let rd = match fname {
        "-" => {
            stdin_buf = stdin_raw();
            &mut stdin_buf as &mut Reader
        }
        _ => {
            file_buf = try!(File::open(&Path::new(fname)));
            &mut file_buf as &mut Reader
        }
    };

    let mut bytes: [u8; 1024 * 1024] = unsafe { mem::uninitialized() };
    loop {
        match rd.read(&mut bytes) {
            Ok(num_bytes) => {
                for &b in bytes[..num_bytes].iter() {
                    crc = crc_update(crc, b);
                }
                size += num_bytes;
            }
            Err(IoError { kind: EndOfFile, .. }) => return Ok((crc_final(crc, size), size)),
            Err(err) => return Err(err)
        }
    }
}

pub fn uumain(args: Vec<String>) -> isize {
    let opts = [
        getopts::optflag("h", "help", "display this help and exit"),
        getopts::optflag("V", "version", "output version information and exit"),
    ];

    let matches = match getopts::getopts(args.tail(), &opts) {
        Ok(m) => m,
        Err(err) => panic!("{}", err),
    };

    if matches.opt_present("help") {
        println!("{} {}", NAME, VERSION);
        println!("");
        println!("Usage:");
        println!("  {} [OPTIONS] [FILE]...", NAME);
        println!("");
        print(getopts::usage("Print CRC and size for each file.", opts.as_slice()).as_slice());
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
        match cksum(fname.as_slice()) {
            Ok((crc, size)) => println!("{} {} {}", crc, size, fname),
            Err(err) => {
                show_error!("'{}' {}", fname, err);
                exit_code = 2;
            }
        }
    }

    exit_code
}
