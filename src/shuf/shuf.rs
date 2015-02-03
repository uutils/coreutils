#![crate_name = "shuf"]
#![feature(collections, core, io, libc, path, rand, rustc_private)]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Arcterus <arcterus@mail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate getopts;
extern crate libc;

use std::cmp;
use std::old_io as io;
use std::old_io::IoResult;
use std::iter::{range_inclusive, RangeInclusive};
use std::rand::{self, Rng};
use std::usize;

#[path = "../common/util.rs"]
#[macro_use]
mod util;

enum Mode {
    Default,
    Echo,
    InputRange(RangeInclusive<usize>)
}

static NAME: &'static str = "shuf";
static VERSION: &'static str = "0.0.1";

pub fn uumain(args: Vec<String>) -> isize {
    let program = args[0].clone();

    let opts = [
        getopts::optflag("e", "echo", "treat each ARG as an input line"),
        getopts::optopt("i", "input-range", "treat each number LO through HI as an input line", "LO-HI"),
        getopts::optopt("n", "head-count", "output at most COUNT lines", "COUNT"),
        getopts::optopt("o", "output", "write result to FILE instead of standard output", "FILE"),
        getopts::optopt("", "random-source", "get random bytes from FILE", "FILE"),
        getopts::optflag("r", "repeat", "output lines can be repeated"),
        getopts::optflag("z", "zero-terminated", "end lines with 0 byte, not newline"),
        getopts::optflag("h", "help", "display this help and exit"),
        getopts::optflag("V", "version", "output version information and exit")
    ];
    let mut matches = match getopts::getopts(args.tail(), &opts) {
        Ok(m) => m,
        Err(f) => {
            crash!(1, "{}", f)
        }
    };
    if matches.opt_present("help") {
        println!("{name} v{version}

Usage:
  {prog} [OPTION]... [FILE]
  {prog} -e [OPTION]... [ARG]...
  {prog} -i LO-HI [OPTION]...\n
{usage}
With no FILE, or when FILE is -, read standard input.",
                 name = NAME, version = VERSION, prog = program,
                 usage = getopts::usage("Write a random permutation of the input lines to standard output.", &opts));
    } else if matches.opt_present("version") {
        println!("{} v{}", NAME, VERSION);
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
                    Err((msg, code)) => {
                        show_error!("{}", msg);
                        return code;
                    }
                }
            }
            None => {
                if echo {
                    Mode::Echo
                } else {
                    if matches.free.len() == 0 {
                        matches.free.push("-".to_string());
                    }
                    Mode::Default
                }
            }
        };
        let repeat = matches.opt_present("repeat");
        let zero = matches.opt_present("zero-terminated");
        let count = match matches.opt_str("head-count") {
            Some(cnt) => match cnt.parse::<usize>() {
                Ok(val) => val,
                Err(e) => {
                    show_error!("'{}' is not a valid count: {}", cnt, e);
                    return 1;
                }
            },
            None => usize::MAX
        };
        let output = matches.opt_str("output");
        let random = matches.opt_str("random-source");
        match shuf(matches.free, mode, repeat, zero, count, output, random) {
            Err(f) => {
                show_error!("{}", f);
                return 1;
            },
            _ => {}
        }
    }

    0
}

fn shuf(input: Vec<String>, mode: Mode, repeat: bool, zero: bool, count: usize, output: Option<String>, random: Option<String>) -> IoResult<()> {
    match mode {
        Mode::Echo => shuf_lines(input, repeat, zero, count, output, random),
        Mode::InputRange(range) => shuf_lines(range.map(|num| num.to_string()).collect(), repeat, zero, count, output, random),
        Mode::Default => {
            let lines: Vec<String> = input.into_iter().flat_map(|filename| {
                let slice = filename.as_slice();
                let mut file_buf;
                let mut stdin_buf;
                let mut file = io::BufferedReader::new(
                    if slice == "-" {
                        stdin_buf = io::stdio::stdin_raw();
                        &mut stdin_buf as &mut Reader
                    } else {
                        file_buf = crash_if_err!(1, io::File::open(&Path::new(slice)));
                        &mut file_buf as &mut Reader
                    }
                );
                let mut lines = vec!();
                for line in file.lines() {
                    let mut line = crash_if_err!(1, line);
                    line.pop();
                    lines.push(line);
                }
                lines.into_iter()
            }).collect();
            shuf_lines(lines, repeat, zero, count, output, random)
        }
    }
}

enum WrappedRng {
    RngFile(rand::reader::ReaderRng<io::File>),
    RngDefault(rand::ThreadRng),
}

impl WrappedRng {
    fn next_u32(&mut self) -> u32 {
        match self {
            &mut WrappedRng::RngFile(ref mut r) => r.next_u32(),
            &mut WrappedRng::RngDefault(ref mut r) => r.next_u32(),
        }
    }
}

fn shuf_lines(mut lines: Vec<String>, repeat: bool, zero: bool, count: usize, outname: Option<String>, random: Option<String>) -> IoResult<()> {
    let mut output = match outname {
        Some(name) => Box::new(io::BufferedWriter::new(try!(io::File::create(&Path::new(name))))) as Box<Writer>,
        None => Box::new(io::stdout()) as Box<Writer>
    };
    let mut rng = match random {
        Some(name) => WrappedRng::RngFile(rand::reader::ReaderRng::new(try!(io::File::open(&Path::new(name))))),
        None => WrappedRng::RngDefault(rand::thread_rng()),
    };
    let mut len = lines.len();
    let max = if repeat { count } else { cmp::min(count, len) };
    for _ in range(0, max) {
        let idx = rng.next_u32() as usize % len;
        try!(write!(output, "{}{}", lines[idx], if zero { '\0' } else { '\n' }));
        if !repeat {
            lines.remove(idx);
            len -= 1;
        }
    }
    Ok(())
}

fn parse_range(input_range: String) -> Result<RangeInclusive<usize>, (String, isize)> {
    let split: Vec<&str> = input_range.as_slice().split('-').collect();
    if split.len() != 2 {
        Err(("invalid range format".to_string(), 1))
    } else {
        let begin = match split[0].parse::<usize>() {
            Ok(m) => m,
            Err(e)=> return Err((format!("{} is not a valid number: {}", split[0], e), 1))
        };
        let end = match split[1].parse::<usize>() {
            Ok(m) => m,
            Err(e)=> return Err((format!("{} is not a valid number: {}", split[1], e), 1))
        };
        Ok(range_inclusive(begin, end))
    }
}
