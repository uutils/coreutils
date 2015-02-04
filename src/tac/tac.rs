#![crate_name = "tac"]
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

#[path = "../common/util.rs"]
#[macro_use]
mod util;

static NAME: &'static str = "tac";
static VERSION: &'static str = "1.0.0";

pub fn uumain(args: Vec<String>) -> isize {
    let program = args[0].clone();

    let opts = [
        getopts::optflag("b", "before", "attach the separator before instead of after"),
        getopts::optflag("r", "regex", "interpret the sequence as a regular expression (NOT IMPLEMENTED)"),
        getopts::optopt("s", "separator", "use STRING as the separator instead of newline", "STRING"),
        getopts::optflag("h", "help", "display this help and exit"),
        getopts::optflag("V", "version", "output version information and exit")
    ];
    let matches = match getopts::getopts(args.tail(), &opts) {
        Ok(m) => m,
        Err(f) => crash!(1, "{}", f)
    };
    if matches.opt_present("help") {
        println!("tac {}", VERSION);
        println!("");
        println!("Usage:");
        println!("  {0} [OPTION]... [FILE]...", program);
        println!("");
        print!("{}", getopts::usage("Write each file to standard output, last line first.", &opts));
    } else if matches.opt_present("version") {
        println!("tac {}", VERSION);
    } else {
        let before = matches.opt_present("b");
        let regex = matches.opt_present("r");
        let separator = match matches.opt_str("s") {
            Some(m) => {
                if m.len() == 0 {
                    crash!(1, "separator cannot be empty")
                } else {
                    m
                }
            }
            None => "\n".to_string()
        };
        let files = if matches.free.is_empty() {
            vec!("-".to_string())
        } else {
            matches.free
        };
        tac(files, before, regex, separator.as_slice());
    }

    0
}

fn tac(filenames: Vec<String>, before: bool, _: bool, separator: &str) {
    for filename in filenames.into_iter() {
        let mut file = io::BufferedReader::new(
            if filename.as_slice() == "-" {
                Box::new(io::stdio::stdin_raw()) as Box<Reader>
            } else {
                let r = crash_if_err!(1, io::File::open(&Path::new(filename)));
                Box::new(r) as Box<Reader>
            }
        );
        let mut data = crash_if_err!(1, file.read_to_string());
        if data.as_slice().ends_with("\n") {
            // removes blank line that is inserted otherwise
            let mut buf = data.to_string();
            let len = buf.len();
            buf.truncate(len - 1);
            data = buf.to_string();
        }
        let split_vec: Vec<&str> = data.as_slice().split_str(separator).collect();
        let rev: String = split_vec.iter().rev().fold(String::new(), |mut a, &b| {
            if before {
               a.push_str(separator);
               a.push_str(b);
            } else {
                a.push_str(b);
                a.push_str(separator);
            }
            a
        });
        print!("{}", rev);
    }
}
