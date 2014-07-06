#![crate_name = "cksum"]
#![feature(macro_rules)]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Michael Gehring <mg@ebfe.org>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate getopts;

use std::io::{BufferedReader, EndOfFile, File, IoError, IoResult, print};
use std::io::stdio::stdin;

#[path="../common/util.rs"]
mod util;

static NAME : &'static str = "cksum";
static VERSION : &'static str = "1.0.0";

fn crc_update(mut crc: u32, input: u8) -> u32 {
    crc ^= input as u32 << 24;

    for _ in range(0u, 8) {
        if crc & 0x80000000 != 0 {
            crc <<= 1;
            crc ^= 0x04c11db7;
        } else {
            crc <<= 1;
        }
    }

    crc
}

fn crc_final(mut crc: u32, mut length: uint) -> u32 {
    while length != 0 {
        crc = crc_update(crc, length as u8);
        length >>= 8;
    }

    !crc
}

fn cksum(fname: &str) -> IoResult<(u32, uint)> {
    let mut crc = 0u32;
    let mut size = 0u;

    let mut rd = try!(open_file(fname));
    loop {
        match rd.read_byte() {
            Ok(b) => {
                crc = crc_update(crc, b);
                size += 1;
            }
            Err(err) =>  {
                return match err {
                    IoError{kind: k, ..} if k == EndOfFile => Ok((crc_final(crc, size), size)),
                    _ => Err(err),
                }
            }
        }
    }
}

fn open_file(name: &str) -> IoResult<Box<Reader>> {
    match name {
        "-" => Ok(box stdin() as Box<Reader>),
        _   => {
            let f = try!(File::open(&Path::new(name)));
            Ok(box BufferedReader::new(f) as Box<Reader>)
        }
    }
}

pub fn uumain(args: Vec<String>) -> int {
    let opts = [
        getopts::optflag("h", "help", "display this help and exit"),
        getopts::optflag("V", "version", "output version information and exit"),
    ];

    let matches = match getopts::getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(err) => fail!("{}", err),
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
