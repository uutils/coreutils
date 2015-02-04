#![crate_name = "comm"]
#![feature(collections, core, io, path, rustc_private)]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Michael Gehring <mg@ebfe.org>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate getopts;

use std::cmp::Ordering;
use std::old_io::{BufferedReader, IoResult, print};
use std::old_io::fs::File;
use std::old_io::stdio::{stdin, StdinReader};
use std::path::Path;

static NAME : &'static str = "comm";
static VERSION : &'static str = "1.0.0";

fn mkdelim(col: usize, opts: &getopts::Matches) -> String {
    let mut s = String::new();
    let delim = match opts.opt_str("output-delimiter") {
        Some(d) => d.clone(),
        None => "\t".to_string(),
    };

    if col > 1 && !opts.opt_present("1") {
        s.push_str(delim.as_slice());
    }
    if col > 2 && !opts.opt_present("2") {
        s.push_str(delim.as_slice());
    }

    s
}

fn ensure_nl(line: String) -> String {
    match line.as_slice().chars().last() {
        Some('\n') => line,
        _ => line + "\n",
    }
}

enum LineReader {
    Stdin(StdinReader),
    FileIn(BufferedReader<File>)
}

impl LineReader {
    fn read_line(&mut self) -> IoResult<String> {
        match self {
            &mut LineReader::Stdin(ref mut r)  => r.read_line(),
            &mut LineReader::FileIn(ref mut r) => r.read_line(),
        }
    }
}

fn comm(a: &mut LineReader, b: &mut LineReader, opts: &getopts::Matches) {

    let delim : Vec<String> = range(0, 4).map(|col| mkdelim(col, opts)).collect();

    let mut ra = a.read_line();
    let mut rb = b.read_line();

    while ra.is_ok() || rb.is_ok() {
        let ord = match (ra.clone(), rb.clone()) {
            (Err(_), Ok(_))  => Ordering::Greater,
            (Ok(_) , Err(_)) => Ordering::Less,
            (Ok(s0), Ok(s1)) => s0.cmp(&s1),
            _ => unreachable!(),
        };

        match ord {
            Ordering::Less => {
                if !opts.opt_present("1") {
                    print!("{}{}", delim[1], ra.map(ensure_nl).unwrap());
                }
                ra = a.read_line();
            }
            Ordering::Greater => {
                if !opts.opt_present("2") {
                    print!("{}{}", delim[2], rb.map(ensure_nl).unwrap());
                }
                rb = b.read_line();
            }
            Ordering::Equal => {
                if !opts.opt_present("3") {
                    print!("{}{}", delim[3], ra.map(ensure_nl).unwrap());
                }
                ra = a.read_line();
                rb = b.read_line();
            }
        }
    }
}

fn open_file(name: &str) -> IoResult<LineReader> {
    match name {
        "-" => Ok(LineReader::Stdin(stdin())),
        _   => {
            let f = try!(std::old_io::File::open(&Path::new(name)));
            Ok(LineReader::FileIn(BufferedReader::new(f)))
        }
    }
}

pub fn uumain(args: Vec<String>) -> isize {
    let opts = [
        getopts::optflag("1", "", "suppress column 1 (lines uniq to FILE1)"),
        getopts::optflag("2", "", "suppress column 2 (lines uniq to FILE2)"),
        getopts::optflag("3", "", "suppress column 3 (lines that appear in both files)"),
        getopts::optopt("", "output-delimiter", "separate columns with STR", "STR"),
        getopts::optflag("h", "help", "display this help and exit"),
        getopts::optflag("V", "version", "output version information and exit"),
    ];

    let matches = match getopts::getopts(args.tail(), &opts) {
        Ok(m) => m,
        Err(err) => panic!("{}", err),
    };

    if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
        return 0;
    }

    if matches.opt_present("help") || matches.free.len() != 2 {
        println!("{} {}", NAME, VERSION);
        println!("");
        println!("Usage:");
        println!("  {} [OPTIONS] FILE1 FILE2", NAME);
        println!("");
        print(getopts::usage("Compare sorted files line by line.", opts.as_slice()).as_slice());
        if matches.free.len() != 2 {
            return 1;
        }
        return 0;
    }


    let mut f1 = open_file(matches.free[0].as_slice()).unwrap();
    let mut f2 = open_file(matches.free[1].as_slice()).unwrap();

    comm(&mut f1, &mut f2, &matches);

    0
}
