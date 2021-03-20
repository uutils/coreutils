// This file is part of the uutils coreutils package.
//
// (c) Michael Gehring <mg@ebfe.org>
//
//  For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) fname

#[macro_use]
extern crate uucore;

use std::fs::File;
use std::io::{self, stdin, BufReader, Read};
use std::path::Path;

// NOTE: CRC_TABLE_LEN *must* be <= 256 as we cast 0..CRC_TABLE_LEN to u8
const CRC_TABLE_LEN: usize = 256;
const CRC_TABLE: [u32; CRC_TABLE_LEN] = generate_crc_table();

const SYNTAX: &str = "[OPTIONS] [FILE]...";
const SUMMARY: &str = "Print CRC and size for each file";
const LONG_HELP: &str = "";

// this is basically a hack to get "loops" to work on Rust 1.33.  Once we update to Rust 1.46 or
// greater, we can just use while loops
macro_rules! unroll {
    (256, |$i:ident| $s:expr) => {{
        unroll!(@ 32, 0 * 32, $i, $s);
        unroll!(@ 32, 1 * 32, $i, $s);
        unroll!(@ 32, 2 * 32, $i, $s);
        unroll!(@ 32, 3 * 32, $i, $s);
        unroll!(@ 32, 4 * 32, $i, $s);
        unroll!(@ 32, 5 * 32, $i, $s);
        unroll!(@ 32, 6 * 32, $i, $s);
        unroll!(@ 32, 7 * 32, $i, $s);
    }};
    (8, |$i:ident| $s:expr) => {{
        unroll!(@ 8, 0, $i, $s);
    }};

    (@ 32, $start:expr, $i:ident, $s:expr) => {{
        unroll!(@ 8, $start + 0 * 8, $i, $s);
        unroll!(@ 8, $start + 1 * 8, $i, $s);
        unroll!(@ 8, $start + 2 * 8, $i, $s);
        unroll!(@ 8, $start + 3 * 8, $i, $s);
    }};
    (@ 8, $start:expr, $i:ident, $s:expr) => {{
        unroll!(@ 4, $start, $i, $s);
        unroll!(@ 4, $start + 4, $i, $s);
    }};
    (@ 4, $start:expr, $i:ident, $s:expr) => {{
        unroll!(@ 2, $start, $i, $s);
        unroll!(@ 2, $start + 2, $i, $s);
    }};
    (@ 2, $start:expr, $i:ident, $s:expr) => {{
        unroll!(@ 1, $start, $i, $s);
        unroll!(@ 1, $start + 1, $i, $s);
    }};
    (@ 1, $start:expr, $i:ident, $s:expr) => {{
        let $i = $start;
        let _ = $s;
    }};
}

const fn generate_crc_table() -> [u32; CRC_TABLE_LEN] {
    let mut table = [0; CRC_TABLE_LEN];

    // NOTE: works on Rust 1.46
    //let mut i = 0;
    //while i < CRC_TABLE_LEN {
    //    table[i] = crc_entry(i as u8) as u32;
    //
    //    i += 1;
    //}
    unroll!(256, |i| {
        table[i] = crc_entry(i as u8) as u32;
    });

    table
}

const fn crc_entry(input: u8) -> u32 {
    let mut crc = (input as u32) << 24;

    // NOTE: this does not work on Rust 1.33, but *does* on 1.46
    //let mut i = 0;
    //while i < 8 {
    //    if crc & 0x8000_0000 != 0 {
    //        crc <<= 1;
    //        crc ^= 0x04c1_1db7;
    //    } else {
    //        crc <<= 1;
    //    }
    //
    //    i += 1;
    //}
    unroll!(8, |_i| {
        let if_cond = crc & 0x8000_0000;
        let if_body = (crc << 1) ^ 0x04c1_1db7;
        let else_body = crc << 1;

        // NOTE: i feel like this is easier to understand than emulating an if statement in bitwise
        //       ops
        let cond_table = [else_body, if_body];

        crc = cond_table[(if_cond != 0) as usize];
    });

    crc
}

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

fn init_byte_array() -> Vec<u8> {
    vec![0; 1024 * 1024]
}

#[inline]
fn cksum(fname: &str) -> io::Result<(u32, usize)> {
    let mut crc = 0u32;
    let mut size = 0usize;

    let file;
    let mut rd: Box<dyn Read> = match fname {
        "-" => Box::new(stdin()),
        _ => {
            file = File::open(&Path::new(fname))?;
            Box::new(BufReader::new(file))
        }
    };

    let mut bytes = init_byte_array();
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

pub fn uumain(args: impl uucore::Args) -> i32 {
    let matches = app!(SYNTAX, SUMMARY, LONG_HELP).parse(args.collect_str());

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
    for fname in &files {
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
