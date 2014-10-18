#![crate_name = "sort"]
#![feature(macro_rules)]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Michael Yin <mikeyin@mikeyin.org>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate getopts;

use std::io::{print, File, BufferedReader};
use std::io::stdio::{stdin_raw, stderr};
use std::str::Chars;

#[path = "../common/util.rs"]
mod util;

static NAME: &'static str = "sort";
static VERSION:  &'static str = "0.0.1";

static DECIMAL_PT: char = '.';
static THOUSANDS_SEP: char = ',';

pub fn uumain(args: Vec<String>) -> int {
    let program = args[0].as_slice();
    let opts = [
        getopts::optflag("n", "numeric-sort", "compare according to string numerical value"),
        getopts::optflag("h", "help", "display this help and exit"),
        getopts::optflag("", "version", "output version information and exit"),
    ];

    let matches = match getopts::getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(f) => crash!(1, "Invalid options\n{}", f)
    };
    if matches.opt_present("help") {
        println!("Usage: {0:s} [OPTION]... [FILE]...", program);
        println!("Write the sorted concatenation of all FILE(s) to standard output.");
        println!("");
        print(getopts::usage("Mandatory arguments for long options are mandatory for short options too.", opts).as_slice());
        println!("");
        println!("With no FILE, or when FILE is -, read standard input.");
        return 0;
    }

    if matches.opt_present("version") {
        println!("sort 1.0.0");
        return 0;
    }

    let mut numeric = false;
    if matches.opt_present("numeric-sort") {
        numeric = true;
    }

    let mut files = matches.free;
    if files.is_empty() {
        /* if no file, default to stdin */
        files.push("-".to_string());
    }
    
    exec(files, numeric);
    
    0
}

fn exec(files: Vec<String>, numeric: bool) {
    for path in files.iter() {
        let (reader, _) = match open(path.as_slice()) {
            Some(x) => x,
            None => continue,
        };
        
        let mut buf_reader = BufferedReader::new(reader);
        let mut lines = Vec::new();

        for line in buf_reader.lines() {
            match line {
                Ok(n) => {
                    lines.push(n);
                },
                _ => break
            }
        }

        if numeric {
            lines.sort_by(frac_compare);
        } else {
            lines.sort();
        }
        for line in lines.iter() {
            print!("{}", line)
        }
    }
}

fn skip_zeros(mut char_a: char, char_iter: &mut Chars, ret: Ordering) -> Ordering {
    char_a = match char_iter.next() { None => 0 as char, Some(t) => t };
    while char_a == '0' {
        char_a = match char_iter.next() { None => return Equal, Some(t) => t };
    }
    if char_a.is_digit() { ret } else { Equal }
}

/// Compares two decimal fractions as strings (n < 1)
/// This requires the strings to start with a decimal, otherwise it's treated as 0
fn frac_compare(a: &String, b: &String) -> Ordering {
    let a_chars = &mut a.as_slice().chars();
    let b_chars = &mut b.as_slice().chars();

    let mut char_a = match a_chars.next() { None => 0 as char, Some(t) => t };
    let mut char_b = match b_chars.next() { None => 0 as char, Some(t) => t };

    if char_a == DECIMAL_PT && char_b == DECIMAL_PT {
        while char_a == char_b {
            char_a = match a_chars.next() { None => 0 as char, Some(t) => t };
            char_b = match b_chars.next() { None => 0 as char, Some(t) => t };
            // hit the end at the same time, they are equal
            if !char_a.is_digit() {
                return Equal;
            }
        }
        if char_a.is_digit() && char_b.is_digit() {
            (char_a as int).cmp(&(char_b as int))
        } else if char_a.is_digit() {
            skip_zeros(char_a, a_chars, Greater)
        } else if char_b.is_digit() {
            skip_zeros(char_b, b_chars, Less)
        } else { Equal }
    } else if char_a == DECIMAL_PT {
        skip_zeros(char_a, a_chars, Greater)
    } else if char_b == DECIMAL_PT {
        skip_zeros(char_b, b_chars, Less)
    } else { Equal }
}


// from cat.rs
fn open<'a>(path: &str) -> Option<(Box<Reader + 'a>, bool)> {
    if path == "-" {
        let stdin = stdin_raw();
        let interactive = stdin.isatty();
        return Some((box stdin as Box<Reader>, interactive));
    }

    match File::open(&std::path::Path::new(path)) {
        Ok(f) => Some((box f as Box<Reader>, false)),
        Err(e) => {
            show_error!("sort: {0:s}: {1:s}", path, e.to_string());
            None
        },
    }
}

