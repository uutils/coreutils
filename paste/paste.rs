#![crate_id(name = "paste", vers = "1.0.0", author = "Arcterus")]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Arcterus <arcterus@mail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

#![feature(macro_rules)]

extern crate getopts;

use std::io;
use std::os;

#[path = "../common/util.rs"]
mod util;

static NAME: &'static str = "paste";
static VERSION: &'static str = "1.0.0";

fn main() {
    let args = os::args();
    let program = args[0].clone();

    let opts = ~[
        getopts::optflag("s", "serial", "paste one file at a time instead of in parallel"),
        getopts::optopt("d", "delimiters", "reuse characters from LIST instead of TABs", "LIST"),
        getopts::optflag("h", "help", "display this help and exit"),
        getopts::optflag("V", "version", "output version information and exit")
    ];
    let matches = match getopts::getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(f) => crash!(1, "{}", f.to_err_msg())
    };
    if matches.opt_present("help") {
        println!("{} {}", NAME, VERSION);
        println!("");
        println!("Usage:");
        println!("  {0:s} [OPTION]... [FILE]...", program);
        println!("");
        print!("{}", getopts::usage("Write lines consisting of the sequentially corresponding lines from each FILE, separated by TABs, to standard output.", opts));
    } else if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
    } else {
        let serial = matches.opt_present("serial");
        let delimiters = match matches.opt_str("delimiters") {
            Some(m) => m,
            None => ~"\t"
        };
        paste(matches.free, serial, delimiters);
    }
}

fn paste(filenames: Vec<~str>, serial: bool, delimiters: ~str) {
    let mut files: ~[io::BufferedReader<io::File>] = filenames.move_iter().map(|name|
        io::BufferedReader::new(crash_if_err!(1, io::File::open(&Path::new(name))))
    ).collect();
    let delimiters: ~[~str] = delimiters.chars().map(|x| x.to_str()).collect();
    let mut delim_count = 0;
    if serial {
        for file in files.mut_iter() {
            let mut output = ~"";
            loop {
                output = output + match file.read_line() {
                    Ok(line) => line.trim_right() + delimiters[delim_count % delimiters.len()],
                    Err(f) => if f.kind == io::EndOfFile {
                        break
                    } else {
                        crash!(1, "{}", f.to_str())
                    }
                };
                delim_count += 1;
            }
            output.pop_char();
            println!("{}", output);
        }
    } else {
        let mut eof = Vec::from_elem(files.len(), false);
        loop {
            let mut output = ~"";
            let mut eof_count = 0;
            for (i, file) in files.mut_iter().enumerate() {
                if *eof.get(i) {
                    eof_count += 1;
                } else {
                    match file.read_line() {
                        Ok(line) => output = output + line.slice_to(line.len() - 1),
                        Err(f) => if f.kind == io::EndOfFile {
                            *eof.get_mut(i) = true;
                            eof_count += 1;
                        } else {
                            crash!(1, "{}", f.to_str());
                        }
                    }
                }
                output = output + delimiters[delim_count % delimiters.len()];
                delim_count += 1;
            }
            if files.len() == eof_count {
                break;
            }
            output.pop_char();
            println!("{}", output);
            delim_count = 0;
        }
    }
}
