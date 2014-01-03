#[crate_id(name="head", vers="1.0.0", author="Alan Andrade")];
/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Alan Andrade <alan.andradec@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 *
 * Synced with: https://raw.github.com/avsm/src/master/usr.bin/head/head.c
 */

extern crate extra;
extern crate getopts;

use std::os;
use std::char;
use std::io::{stdin};
use std::io::BufferedReader;
use std::io::fs::File;
use std::path::Path;
use getopts::{optopt, optflag, getopts, usage};

static PROGRAM: &'static str = "head";

fn main () {
    let args = os::args();

    let mut options = args.tail().to_owned();
    let mut line_count = 10u;

    // handle obsolete -number syntax
    match obsolete(&mut options) {
        Some(n) => { line_count = n },
        None => {}
    }

    let possible_options = [
        optopt("n", "number", "Number of lines to print", "n"),
        optflag("h", "help", "help"),
        optflag("V", "version", "version")
    ];

    let given_options = match getopts(options, possible_options) {
        Ok (m) => { m }
        Err(_) => {
            println!("{:s}", usage(PROGRAM, possible_options));
            return
        }
    };

    if given_options.opt_present("h") {
        println!("{:s}", usage(PROGRAM, possible_options));
        return;
    }
    if given_options.opt_present("V") { version(); return }

    match given_options.opt_str("n") {
        Some(n) => {
            match from_str(n) {
                Some(m) => { line_count = m }
                None => {}
            }
        }
        None => {}
    };

    let files = given_options.free;

    if files.is_empty() {
        let mut buffer = BufferedReader::new(stdin());
        head(&mut buffer, line_count);
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
            head(&mut buffer, line_count);
        }
    }
}

// It searches for an option in the form of -123123
//
// In case is found, the options vector will get rid of that object so that
// getopts works correctly.
fn obsolete (options: &mut ~[~str]) -> Option<uint> {
    let mut a = 0;
    let b = options.len();
    let mut current;

    while a < b {
        current = options[a].clone();

        if current.len() > 1 && current[0] == '-' as u8 {
            let len = current.len();
            for pos in range(1, len) {
                // Ensure that the argument is only made out of digits
                if !char::is_digit(current.char_at(pos)) { break; }

                // If this is the last number
                if pos == len - 1 {
                    options.remove(a);
                    let number : Option<uint> = from_str(current.slice(1,len));
                    return Some(number.unwrap());
                }
            }
        }

        a += 1;
    };

    None
}

fn head<T: Reader> (reader: &mut BufferedReader<T>, line_count:uint) {
    for line in reader.lines().take(line_count) { print!("{:s}", line); }
}

fn version () {
    println!("head version 1.0.0");
}
