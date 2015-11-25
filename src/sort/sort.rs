#![crate_name = "sort"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Michael Yin <mikeyin@mikeyin.org>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

#![allow(dead_code)]

extern crate getopts;
extern crate libc;

#[macro_use]
extern crate uucore;

use libc::STDIN_FILENO;
use libc::{c_int, isatty};
use std::cmp::Ordering;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, stdin, Write};
use std::path::Path;

static NAME: &'static str = "sort";
static VERSION: &'static str = "0.0.1";

static DECIMAL_PT: char = '.';
static THOUSANDS_SEP: char = ',';

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = getopts::Options::new();

    opts.optflag("n", "numeric-sort", "compare according to string numerical value");
    opts.optflag("H", "human-readable-sort", "compare according to human readable sizes, eg 1M > 100k");
    opts.optflag("r", "reverse", "reverse the output");
    opts.optflag("h", "help", "display this help and exit");
    opts.optflag("", "version", "output version information and exit");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => crash!(1, "Invalid options\n{}", f)
    };
    if matches.opt_present("help") {
        let msg = format!("{0} {1}

Usage:
 {0} [OPTION]... [FILE]...

Write the sorted concatenation of all FILE(s) to standard output.

Mandatory arguments for long options are mandatory for short options too.

With no FILE, or when FILE is -, read standard input.", NAME, VERSION);
        print!("{}", opts.usage(&msg));
        return 0;
    }

    if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
        return 0;
    }

    let numeric = matches.opt_present("numeric-sort");
    let human_readable = matches.opt_present("human-readable-sort");
    let reverse = matches.opt_present("reverse");

    let mut files = matches.free;
    if files.is_empty() {
        /* if no file, default to stdin */
        files.push("-".to_string());
    }

    exec(files, numeric, human_readable, reverse);

    0
}

fn exec(files: Vec<String>, numeric: bool, human_readable: bool, reverse: bool) {
    for path in files.iter() {
        let (reader, _) = match open(path) {
            Some(x) => x,
            None => continue,
        };

        let buf_reader = BufReader::new(reader);
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
            lines.sort_by(numeric_compare);
        } else if human_readable {
            lines.sort_by(human_readable_size_compare);
        } else {
            lines.sort();
        }

        let iter = lines.iter();
        if reverse {
            print_sorted(iter.rev());
        } else {
            print_sorted(iter)
        };
    }
}

/// Parse the beginning string into an f64, returning -inf instead of NaN on errors.
fn permissive_f64_parse(a: &String) -> f64{
    //Maybe should be split on non-digit, but then 10e100 won't parse properly.
    //On the flip side, this will give NEG_INFINITY for "1,234", which might be OK
    //because there's no way to handle both CSV and thousands separators without a new flag.
    //GNU sort treats "1,234" as "1" in numeric, so maybe it's fine.
    let sa: &str = a.split_whitespace().next().unwrap();
    match sa.parse::<f64>() {
        Ok(a) => a,
        Err(_) => std::f64::NEG_INFINITY
    }
}

/// Compares two floating point numbers, with errors being assumned to be -inf.
/// Stops coercing at the first whitespace char, so 1e2 will parse as 100 but 
/// 1,000 will parse as -inf.
fn numeric_compare(a: &String, b: &String) -> Ordering {
    let fa = permissive_f64_parse(a);
    let fb = permissive_f64_parse(b);
    //f64::cmp isn't implemented because NaN messes with it
    //but we sidestep that with permissive_f64_parse so just fake it
    if fa > fb {
        return Ordering::Greater;
    }
    else if fa < fb {
        return Ordering::Less;
    }
    else {
        return Ordering::Equal;
    }
}

fn human_readable_convert(a: &String) -> f64 {
    let int_iter = a.chars();
    let suffix_iter = a.chars();
    let int_str: String = int_iter.take_while(|c| c.is_numeric()).collect();
    let suffix = suffix_iter.skip_while(|c| c.is_numeric()).next();
    let int_part = match int_str.parse::<f64>() {
        Ok(i) => i,
        Err(_) => -1f64
    } as f64;
    let suffix: f64 = match suffix.unwrap_or('\0') {
        'K' => 1000f64,
        'M' => 1E6,
        'G' => 1E9,
        'T' => 1E12,
        'P' => 1E15,
        _ => 1f64
    };
    return int_part * suffix;
}

/// Compare two strings as if they are human readable sizes.
/// AKA 1M > 100k
fn human_readable_size_compare(a: &String, b: &String) -> Ordering {
    let fa = human_readable_convert(a);
    let fb = human_readable_convert(b);
    if fa > fb {
        return Ordering::Greater;
    }
    else if fa < fb {
        return Ordering::Less;
    }
    else {
        return Ordering::Equal;
    }

}

#[inline(always)]
fn print_sorted<S, T: Iterator<Item=S>>(iter: T) where S: std::fmt::Display {
    for line in iter {
        println!("{}", line);
    }
}

// from cat.rs
fn open<'a>(path: &str) -> Option<(Box<Read + 'a>, bool)> {
    if path == "-" {
        let stdin = stdin();
        let interactive = unsafe { isatty(STDIN_FILENO) } != 0 as c_int;
        return Some((Box::new(stdin) as Box<Read>, interactive));
    }

    match File::open(Path::new(path)) {
        Ok(f) => Some((Box::new(f) as Box<Read>, false)),
        Err(e) => {
            show_error!("sort: {0}: {1}", path, e.to_string());
            None
        },
    }
}

#[allow(dead_code)]
fn main() {
    std::process::exit(uumain(std::env::args().collect()));
}
