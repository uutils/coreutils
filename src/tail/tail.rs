#![crate_name = "tail"]
/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Morten Olsen Lysgaard <morten@lysgaard.no>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 *
 */

#![feature(macro_rules)]

extern crate getopts;

use std::char;
use std::io::{stdin};
use std::io::BufferedReader;
use std::io::fs::File;
use std::path::Path;
use std::str::from_utf8;
use getopts::{optopt, optflag, getopts, usage};
use std::collections::Deque;
use std::collections::ringbuf::RingBuf;
use std::io::timer::sleep;
use std::time::duration::Duration;

#[path = "../common/util.rs"]
mod util;

static NAME: &'static str = "tail";
static VERSION: &'static str = "0.0.1";

pub fn uumain(args: Vec<String>) -> int {
    let mut beginning = false;
    let mut lines = true;
    let mut byte_count = 0u;
    let mut line_count = 10u;
    let mut sleep_msec = 1000u64;

    // handle obsolete -number syntax
    let options = match obsolete(args.tail()) {
        (args, Some(n)) => { line_count = n; args },
        (args, None) => args
    };

    let args = options;

    let possible_options = [
        optopt("c", "bytes", "Number of bytes to print", "k"),
        optopt("n", "lines", "Number of lines to print", "k"),
        optflag("f", "follow", "Print the file as it grows"),
        optopt("s", "sleep-interval", "Number or seconds to sleep between polling the file when running with -f", "n"),
        optflag("h", "help", "help"),
        optflag("V", "version", "version"),
    ];

    let given_options = match getopts(args.as_slice(), possible_options) {
        Ok (m) => { m }
        Err(_) => {
            println!("{:s}", usage(NAME, possible_options));
            return 1;
        }
    };

    if given_options.opt_present("h") {
        println!("{:s}", usage(NAME, possible_options));
        return 0;
    }
    if given_options.opt_present("V") { version(); return 0 }

    let follow = given_options.opt_present("f");
    if follow {
        match given_options.opt_str("s") {
            Some(n) => {
                let parsed: Option<u64> = from_str(n.as_slice());
                match parsed {
                    Some(m) => { sleep_msec = m * 1000 }
                    None => {}
                }
            }
            None => {}
        };
    }

    match given_options.opt_str("n") {
        Some(n) => {
            let mut slice = n.as_slice();
            if slice.len() > 0 && slice.char_at(0) == '+' {
                beginning = true;
                slice = slice.slice_from(1);
            }
            line_count = match from_str(slice) {
                Some(m) => m,
                None => {
                    show_error!("invalid number of lines ({})", slice);
                    return 1;
                }
            };
        }
        None => match given_options.opt_str("c") {
            Some(n) => {
                let mut slice = n.as_slice();
                if slice.len() > 0 && slice.char_at(0) == '+' {
                    beginning = true;
                    slice = slice.slice_from(1);
                }
                byte_count = match from_str(slice) {
                    Some(m) => m,
                    None => {
                        show_error!("invalid number of bytes ({})", slice);
                        return 1;
                    }
                };
                lines = false;
            }
            None => { }
        }
    };

    let files = given_options.free;

    if files.is_empty() {
        let mut buffer = BufferedReader::new(stdin());
        tail(&mut buffer, line_count, byte_count, beginning, lines, follow, sleep_msec);
    } else {
        let mut multiple = false;
        let mut firstime = true;

        if files.len() > 1 {
            multiple = true;
        }


        for file in files.iter() {
            if multiple {
                if !firstime { println!(""); }
                println!("==> {:s} <==", file.as_slice());
            }
            firstime = false;

            let path = Path::new(file.as_slice());
            let reader = File::open(&path).unwrap();
            let mut buffer = BufferedReader::new(reader);
            tail(&mut buffer, line_count, byte_count, beginning, lines, follow, sleep_msec);
        }
    }

    0
}

// It searches for an option in the form of -123123
//
// In case is found, the options vector will get rid of that object so that
// getopts works correctly.
fn obsolete(options: &[String]) -> (Vec<String>, Option<uint>) {
    let mut options: Vec<String> = options.to_vec();
    let mut a = 0;
    let b = options.len();

    while a < b {
        let current = options[a].clone();
        let current = current.as_bytes();

        if current.len() > 1 && current[0] == '-' as u8 {
            let len = current.len();
            for pos in range(1, len) {
                // Ensure that the argument is only made out of digits
                if !char::is_digit(current[pos] as char) { break; }

                // If this is the last number
                if pos == len - 1 {
                    options.remove(a);
                    let number: Option<uint> = from_str(from_utf8(current.slice(1,len)).unwrap());
                    return (options, Some(number.unwrap()));
                }
            }
        }

        a += 1;
    };

    (options, None)
}

fn tail<T: Reader>(reader: &mut BufferedReader<T>, line_count: uint, byte_count: uint, beginning: bool, lines: bool, follow: bool, sleep_msec: u64) {
    if lines {
        tail_lines(reader, line_count, beginning);
    } else {
        tail_bytes(reader, byte_count, beginning);
    }

    // if we follow the file, sleep a bit and print the rest if the file has grown.
    while follow {
        sleep(Duration::milliseconds(sleep_msec as i64));
        for io_line in reader.lines() {
            match io_line {
                Ok(line) => print!("{}", line),
                Err(err) => fail!(err)
            }
        }
    }
}

#[inline]
fn tail_lines<T: Reader>(reader: &mut BufferedReader<T>, mut line_count: uint, beginning: bool) {
    // read through each line and store them in a ringbuffer that always contains
    // line_count lines. When reaching the end of file, output the lines in the
    // ringbuf.
    let mut ringbuf: RingBuf<String> = RingBuf::new();
    let mut lines = reader.lines().skip(
        if beginning {
            let temp = line_count;
            line_count = ::std::uint::MAX;
            temp - 1
        } else {
            0
        }
    );
    for io_line in lines {
        match io_line {
            Ok(line) => {
                if line_count <= ringbuf.len() {
                    ringbuf.pop_front();
                }
                ringbuf.push(line);
            }
            Err(err) => fail!(err)
        }
    }
    for line in ringbuf.iter() {
        print!("{}", line);
    }
}

#[inline]
fn tail_bytes<T: Reader>(reader: &mut BufferedReader<T>, mut byte_count: uint, beginning: bool) {
    let mut ringbuf: RingBuf<u8> = RingBuf::new();
    let mut bytes = reader.bytes().skip(
        if beginning {
            let temp = byte_count;
            byte_count = ::std::uint::MAX;
            temp - 1
        } else {
            0
        }
    );
    for io_byte in bytes {
        match io_byte {
            Ok(byte) => {
                if byte_count <= ringbuf.len() {
                    ringbuf.pop_front();
                }
                ringbuf.push(byte);
            }
            Err(err) => fail!(err)
        }
    }
    for byte in ringbuf.iter() {
        print!("{}", byte);
    }
}

fn version () {
    println!("{} v{}", NAME, VERSION);
}
