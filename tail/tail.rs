#![crate_id(name="tail", vers="1.0.0", author="Morten Olsen Lysgaard")]
/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Morten Olsen Lysgaard <morten@lysgaard.no>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 *
 */

extern crate getopts;

use std::os;
use std::char;
use std::io::{stdin};
use std::io::BufferedReader;
use std::io::fs::File;
use std::path::Path;
use getopts::{optopt, optflag, getopts, usage};
use std::collections::Deque;
use std::collections::ringbuf::RingBuf;
use std::io::timer::sleep;

static PROGRAM: &'static str = "tail";

#[allow(dead_code)]
fn main() { os::set_exit_status(uumain(os::args())); }

pub fn uumain(args: Vec<String>) -> int {
    let mut line_count = 10u;
    let mut sleep_sec = 1000u64;

    // handle obsolete -number syntax
    let options = match obsolete(args.tail()) {
        (args, Some(n)) => { line_count = n; args },
        (args, None) => args
    };

    let args = options;

    let possible_options = [
        optopt("n", "number", "Number of lines to print", "n"),
        optflag("f", "follow", "Print the file as it grows"),
        optopt("s", "sleep-interval", "Number or seconds to sleep between polling the file when running with -f", "n"),
        optflag("h", "help", "help"),
        optflag("V", "version", "version"),
    ];

    let given_options = match getopts(args.as_slice(), possible_options) {
        Ok (m) => { m }
        Err(_) => {
            println!("{:s}", usage(PROGRAM, possible_options));
            return 1;
        }
    };

    if given_options.opt_present("h") {
        println!("{:s}", usage(PROGRAM, possible_options));
        return 0;
    }
    if given_options.opt_present("V") { version(); return 0 }

    let follow = given_options.opt_present("f");
    if follow {
        match given_options.opt_str("s") {
            Some(n) => {
                let parsed : Option<u64> = from_str(n.as_slice());
                match parsed {
                    Some(m) => { sleep_sec = m*1000 }
                    None => {}
                }
            }
            None => {}
        };
    }

    match given_options.opt_str("n") {
        Some(n) => {
            match from_str(n.as_slice()) {
                Some(m) => { line_count = m }
                None => {}
            }
        }
        None => {}
    };

    let files = given_options.free;

    if files.is_empty() {
        let mut buffer = BufferedReader::new(stdin());
        tail(&mut buffer, line_count, follow, sleep_sec);
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
            tail(&mut buffer, line_count, follow, sleep_sec);
        }
    }
    
    0
}

// It searches for an option in the form of -123123
//
// In case is found, the options vector will get rid of that object so that
// getopts works correctly.
fn obsolete (options: &[String]) -> (Vec<String>, Option<uint>) {
    let mut options: Vec<String> = Vec::from_slice(options);
    let mut a = 0;
    let b = options.len();

    while a < b {
        let current = options.get(a).clone();
        let current = current.as_slice();

        if current.len() > 1 && current[0] == '-' as u8 {
            let len = current.len();
            for pos in range(1, len) {
                // Ensure that the argument is only made out of digits
                if !char::is_digit(current.char_at(pos)) { break; }

                // If this is the last number
                if pos == len - 1 {
                    options.remove(a);
                    let number : Option<uint> = from_str(current.slice(1,len));
                    return (options, Some(number.unwrap()));
                }
            }
        }

        a += 1;
    };

    (options, None)
}

fn tail<T: Reader> (reader: &mut BufferedReader<T>, line_count:uint, follow:bool, sleep_sec:u64) {
    // read through each line and store them in a ringbuffer that always contains
    // line_count lines. When reaching the end of file, output the lines in the
    // ringbuf.
    let mut ringbuf : RingBuf<String> = RingBuf::new();
    for io_line in reader.lines(){
        match io_line {
            Ok(line) => {
                if line_count<=ringbuf.len(){
                  ringbuf.pop_front();
                }
                ringbuf.push_back(line);
            }
            Err(err) => fail!(err)
        }
    }
    for line in ringbuf.iter() {
        print!("{}", line);
    }

    // if we follow the file, sleep a bit and print the rest if the file has grown.
    while follow {
        sleep(sleep_sec);
        for io_line in reader.lines() {
            match io_line {
                Ok(line) => print!("{}", line),
                Err(err) => fail!(err)
            }
        }
    }
}

fn version () {
    println!("tail version 0.0.1");
}
