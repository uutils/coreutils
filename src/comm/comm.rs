#![crate_name = "comm"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Michael Gehring <mg@ebfe.org>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate getopts;

use getopts::Options;
use std::cmp::Ordering;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read, stdin, Stdin};
use std::path::Path;

static NAME: &'static str = "comm";
static VERSION: &'static str = "1.0.0";

fn mkdelim(col: usize, opts: &getopts::Matches) -> String {
    let mut s = String::new();
    let delim = match opts.opt_str("output-delimiter") {
        Some(d) => d.clone(),
        None => "\t".to_string(),
    };

    if col > 1 && !opts.opt_present("1") {
        s.push_str(delim.as_ref());
    }
    if col > 2 && !opts.opt_present("2") {
        s.push_str(delim.as_ref());
    }

    s
}

fn ensure_nl(line: &mut String) {
    match line.chars().last() {
        Some('\n') => (),
        _ => line.push_str("\n"),
    }
}

enum LineReader {
    Stdin(Stdin),
    FileIn(BufReader<File>),
}

impl LineReader {
    fn read_line(&mut self, buf: &mut String) -> io::Result<usize> {
        match self {
            &mut LineReader::Stdin(ref mut r) => r.read_line(buf),
            &mut LineReader::FileIn(ref mut r) => r.read_line(buf),
        }
    }
}

fn comm(a: &mut LineReader, b: &mut LineReader, opts: &getopts::Matches) {

    let delim: Vec<String> = (0..4).map(|col| mkdelim(col, opts)).collect();

    let mut ra = &mut String::new();
    let mut na = a.read_line(ra);
    let mut rb = &mut String::new();
    let mut nb = b.read_line(rb);

    while na.is_ok() || nb.is_ok() {
        let ord = match (na.is_ok(), nb.is_ok()) {
            (false, true) => Ordering::Greater,
            (true , false) => Ordering::Less,
            (true , true) => match (&na, &nb) {
                (&Ok(0), _) => Ordering::Greater,
                (_, &Ok(0)) => Ordering::Less,
                _ => ra.cmp(&rb),
            },
            _ => unreachable!(),
        };

        match ord {
            Ordering::Less => {
                if !opts.opt_present("1") {
                    ensure_nl(ra);
                    print!("{}{}", delim[1], ra);
                }
                na = a.read_line(ra);
            }
            Ordering::Greater => {
                if !opts.opt_present("2") {
                    ensure_nl(rb);
                    print!("{}{}", delim[2], rb);
                }
                nb = b.read_line(rb);
            }
            Ordering::Equal => {
                if !opts.opt_present("3") {
                    ensure_nl(ra);
                    print!("{}{}", delim[3], ra);
                }
                na = a.read_line(ra);
                nb = b.read_line(rb);
            }
        }
    }
}

fn open_file(name: &str) -> io::Result<LineReader> {
    match name {
        "-" => Ok(LineReader::Stdin(stdin())),
        _ => {
            let f = try!(File::open(&Path::new(name)));
            Ok(LineReader::FileIn(BufReader::new(f)))
        }
    }
}

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = Options::new();
    opts.optflag("1", "", "suppress column 1 (lines uniq to FILE1)");
    opts.optflag("2", "", "suppress column 2 (lines uniq to FILE2)");
    opts.optflag("3",
                 "",
                 "suppress column 3 (lines that appear in both files)");
    opts.optopt("",
                "output-delimiter",
                "separate columns with STR",
                "STR");
    opts.optflag("h", "help", "display this help and exit");
    opts.optflag("V", "version", "output version information and exit");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(err) => panic!("{}", err),
    };

    if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
        return 0;
    }

    if matches.opt_present("help") || matches.free.len() != 2 {
        let msg = format!("{0} {1}

Usage:
  {0} [OPTIONS] FILE1 FILE2

Compare sorted files line by line.",
                          NAME,
                          VERSION);

        print!("{}", opts.usage(&msg));

        if matches.free.len() != 2 {
            return 1;
        }
        return 0;
    }

    let mut f1 = open_file(matches.free[0].as_ref()).unwrap();
    let mut f2 = open_file(matches.free[1].as_ref()).unwrap();

    comm(&mut f1, &mut f2, &matches);

    0
}
