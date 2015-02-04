#![crate_name = "paste"]
#![feature(collections, core, io, libc, path, rustc_private)]

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

use std::old_io as io;
use std::iter::repeat; 

#[path = "../common/util.rs"]
#[macro_use]
mod util;

static NAME: &'static str = "paste";
static VERSION: &'static str = "1.0.0";

pub fn uumain(args: Vec<String>) -> isize {
    let program = args[0].clone();

    let opts = [
        getopts::optflag("s", "serial", "paste one file at a time instead of in parallel"),
        getopts::optopt("d", "delimiters", "reuse characters from LIST instead of TABs", "LIST"),
        getopts::optflag("h", "help", "display this help and exit"),
        getopts::optflag("V", "version", "output version information and exit")
    ];
    let matches = match getopts::getopts(args.tail(), &opts) {
        Ok(m) => m,
        Err(f) => crash!(1, "{}", f)
    };
    if matches.opt_present("help") {
        println!("{} {}", NAME, VERSION);
        println!("");
        println!("Usage:");
        println!("  {0} [OPTION]... [FILE]...", program);
        println!("");
        print!("{}", getopts::usage("Write lines consisting of the sequentially corresponding lines from each FILE, separated by TABs, to standard output.", &opts));
    } else if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
    } else {
        let serial = matches.opt_present("serial");
        let delimiters = match matches.opt_str("delimiters") {
            Some(m) => m,
            None => "\t".to_string()
        };
        paste(matches.free, serial, delimiters.as_slice());
    }

    0
}

fn paste(filenames: Vec<String>, serial: bool, delimiters: &str) {
    let mut files: Vec<io::BufferedReader<Box<Reader>>> = filenames.into_iter().map(|name|
        io::BufferedReader::new(
            if name.as_slice() == "-" {
                Box::new(io::stdio::stdin_raw()) as Box<Reader>
            } else {
                let r = crash_if_err!(1, io::File::open(&Path::new(name)));
                Box::new(r) as Box<Reader>
            }
        )
    ).collect();
    let delimiters: Vec<String> = delimiters.chars().map(|x| x.to_string()).collect();
    let mut delim_count = 0;
    if serial {
        for file in files.iter_mut() {
            let mut output = String::new();
            loop {
                match file.read_line() {
                    Ok(line) => {
                        output.push_str(line.as_slice().trim_right());
                        output.push_str(delimiters[delim_count % delimiters.len()].as_slice());
                    }
                    Err(f) => if f.kind == io::EndOfFile {
                        break
                    } else {
                        crash!(1, "{}", f.to_string())
                    }
                }
                delim_count += 1;
            }
            println!("{}", output.as_slice().slice_to(output.len() - 1));
        }
    } else {
        let mut eof : Vec<bool> = repeat(false).take(files.len()).collect();
        loop {
            let mut output = "".to_string();
            let mut eof_count = 0;
            for (i, file) in files.iter_mut().enumerate() {
                if eof[i] {
                    eof_count += 1;
                } else {
                    match file.read_line() {
                        Ok(line) => output.push_str(&line.as_slice()[..line.len() - 1]),
                        Err(f) => if f.kind == io::EndOfFile {
                            eof[i] = true;
                            eof_count += 1;
                        } else {
                            crash!(1, "{}", f.to_string());
                        }
                    }
                }
                output.push_str(delimiters[delim_count % delimiters.len()].as_slice());
                delim_count += 1;
            }
            if files.len() == eof_count {
                break;
            }
            println!("{}", output.as_slice().slice_to(output.len() - 1));
            delim_count = 0;
        }
    }
}
