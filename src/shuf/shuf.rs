#![crate_name = "uu_shuf"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Arcterus <arcterus@mail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate getopts;
extern crate rand;

#[macro_use]
extern crate uucore;

use rand::Rng;
use std::fs::File;
use std::io::{stdin, stdout, BufReader, BufWriter, Read, Write};
use std::usize::MAX as MAX_USIZE;

enum Mode {
    Default,
    Echo,
    InputRange((usize, usize))
}

static NAME: &'static str = "shuf";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = getopts::Options::new();
    opts.optflag("e", "echo", "treat each ARG as an input line");
    opts.optopt("i", "input-range", "treat each number LO through HI as an input line", "LO-HI");
    opts.optopt("n", "head-count", "output at most COUNT lines", "COUNT");
    opts.optopt("o", "output", "write result to FILE instead of standard output", "FILE");
    opts.optopt("", "random-source", "get random bytes from FILE", "FILE");
    opts.optflag("r", "repeat", "output lines can be repeated");
    opts.optflag("z", "zero-terminated", "end lines with 0 byte, not newline");
    opts.optflag("h", "help", "display this help and exit");
    opts.optflag("V", "version", "output version information and exit");
    let mut matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            crash!(1, "{}", f)
        }
    };
    if matches.opt_present("help") {
        let msg = format!("{0} {1}

Usage:
  {0} [OPTION]... [FILE]
  {0} -e [OPTION]... [ARG]...
  {0} -i LO-HI [OPTION]...

Write a random permutation of the input lines to standard output.
With no FILE, or when FILE is -, read standard input.", NAME, VERSION);
        print!("{}", opts.usage(&msg));
    } else if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
    } else {
        let echo = matches.opt_present("echo");
        let mode = match matches.opt_str("input-range") {
            Some(range) => {
                if echo {
                    show_error!("cannot specify more than one mode");
                    return 1;
                }
                match parse_range(range) {
                    Ok(m) => Mode::InputRange(m),
                    Err(msg) => {
                        crash!(1, "{}", msg);
                    },
                }
            }
            None => {
                if echo {
                    Mode::Echo
                } else {
                    if matches.free.is_empty() {
                        matches.free.push("-".to_owned());
                    } else if matches.free.len() > 1 {
                        show_error!("extra operand '{}'", &matches.free[1][..]);
                    }
                    Mode::Default
                }
            }
        };
        let repeat = matches.opt_present("repeat");
        let sep = if matches.opt_present("zero-terminated") {
            0x00 as u8
        } else {
            0x0a as u8
        };
        let count = match matches.opt_str("head-count") {
            Some(cnt) => match cnt.parse::<usize>() {
                Ok(val) => val,
                Err(e) => {
                    show_error!("'{}' is not a valid count: {}", cnt, e);
                    return 1;
                }
            },
            None => MAX_USIZE,
        };
        let output = matches.opt_str("output");
        let random = matches.opt_str("random-source");

        match mode {
            Mode::Echo => {
                // XXX: this doesn't correctly handle non-UTF-8 cmdline args
                let mut evec = matches.free.iter().map(|a| a.as_bytes()).collect::<Vec<&[u8]>>();
                find_seps(&mut evec, sep);
                shuf_bytes(&mut evec, repeat, count, sep, output, random);
            },
            Mode::InputRange((b, e)) => {
                let rvec = (b..e).map(|x| format!("{}", x)).collect::<Vec<String>>();
                let mut rvec = rvec.iter().map(|a| a.as_bytes()).collect::<Vec<&[u8]>>();
                shuf_bytes(&mut rvec, repeat, count, sep, output, random);
            },
            Mode::Default => {
                let fdata = read_input_file(&matches.free[0][..]);
                let mut fdata = vec!(&fdata[..]);
                find_seps(&mut fdata, sep);
                shuf_bytes(&mut fdata, repeat, count, sep, output, random);
            }
        }
    }

    0
}

fn read_input_file(filename: &str) -> Vec<u8> {
    let mut file = BufReader::new(
        if filename == "-" {
            Box::new(stdin()) as Box<Read>
        } else {
            match File::open(filename) {
                Ok(f) => Box::new(f) as Box<Read>,
                Err(e) => crash!(1, "failed to open '{}': {}", filename, e),
            }
        });

    let mut data = Vec::new();
    match file.read_to_end(&mut data) {
        Err(e) => crash!(1, "failed reading '{}': {}", filename, e),
        Ok(_) => (),
    };

    data
}

fn find_seps(data: &mut Vec<&[u8]>, sep: u8) {
    // need to use for loop so we don't borrow the vector as we modify it in place
    // basic idea:
    // * We don't care about the order of the result. This lets us slice the slices
    //   without making a new vector.
    // * Starting from the end of the vector, we examine each element.
    // * If that element contains the separator, we remove it from the vector,
    //   and then sub-slice it into slices that do not contain the separator.
    // * We maintain the invariant throughout that each element in the vector past
    //   the ith element does not have any separators remaining.
    for i in (0..data.len()).rev() {
        if data[i].contains(&sep) {
            let this = data.swap_remove(i);
            let mut p = 0;
            let mut i = 1;
            loop {
                if i == this.len() {
                    break;
                }

                if this[i] == sep {
                    data.push(&this[p..i]);
                    p = i + 1;
                }
                i += 1;
            }
            if p < this.len() {
                data.push(&this[p..i]);
            }
        }
    }
}

fn shuf_bytes(input: &mut Vec<&[u8]>, repeat: bool, count: usize, sep: u8, output: Option<String>, random: Option<String>) {
    let mut output = BufWriter::new(
        match output {
            None => Box::new(stdout()) as Box<Write>,
            Some(s) => match File::create(&s[..]) {
                Ok(f) => Box::new(f) as Box<Write>,
                Err(e) => crash!(1, "failed to open '{}' for writing: {}", &s[..], e),
            },
        });

    let mut rng = match random {
        Some(r) => WrappedRng::RngFile(rand::read::ReadRng::new(match File::open(&r[..]) {
            Ok(f) => f,
            Err(e) => crash!(1, "failed to open random source '{}': {}", &r[..], e),
        })),
        None => WrappedRng::RngDefault(rand::thread_rng()),
    };

    // we're generating a random usize. To keep things fair, we take this number mod ceil(log2(length+1))
    let mut len_mod = 1;
    let mut len = input.len();
    while len > 0 {
        len >>= 1;
        len_mod <<= 1;
    }
    drop(len);

    let mut count = count;
    while count > 0 && !input.is_empty() {
        let mut r = input.len();
        while r >= input.len() {
            r = rng.next_usize() % len_mod;
        }

        // write the randomly chosen value and the separator
        output.write_all(input[r]).unwrap_or_else(|e| crash!(1, "write failed: {}", e));
        output.write_all(&[sep]).unwrap_or_else(|e| crash!(1, "write failed: {}", e));

        // if we do not allow repeats, remove the chosen value from the input vector
        if !repeat {
            // shrink the mask if we will drop below a power of 2
            if input.len() % 2 == 0 && len_mod > 2 {
                len_mod >>= 1;
            }
            input.swap_remove(r);
        }

        count -= 1;
    }
}

fn parse_range(input_range: String) -> Result<(usize, usize), String> {
    let split: Vec<&str> = input_range.split('-').collect();
    if split.len() != 2 {
        Err("invalid range format".to_owned())
    } else {
        let begin = match split[0].parse::<usize>() {
            Ok(m) => m,
            Err(e)=> return Err(format!("{} is not a valid number: {}", split[0], e)),
        };
        let end = match split[1].parse::<usize>() {
            Ok(m) => m,
            Err(e)=> return Err(format!("{} is not a valid number: {}", split[1], e)),
        };
        Ok((begin, end + 1))
    }
}

enum WrappedRng {
    RngFile(rand::read::ReadRng<File>),
    RngDefault(rand::ThreadRng),
}

impl WrappedRng {
    fn next_usize(&mut self) -> usize {
        match *self {
            WrappedRng::RngFile(ref mut r) => r.next_u32() as usize,
            WrappedRng::RngDefault(ref mut r) => r.next_u32() as usize,
        }
    }
}
