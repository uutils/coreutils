// This file is part of the uutils coreutils package.
//
// (c) Michael Gehring <mg@ebfe.org>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) delim mkdelim

#[macro_use]
extern crate uucore;

use std::cmp::Ordering;
use std::fs::File;
use std::io::{self, stdin, BufRead, BufReader, Stdin};
use std::path::Path;

use clap::{App, Arg, ArgMatches};

static VERSION: &str = env!("CARGO_PKG_VERSION");
static ABOUT: &str = "compare two sorted files line by line";
static LONG_HELP: &str = "";

mod options {
    pub const COLUMN_1: &str = "1";
    pub const COLUMN_2: &str = "2";
    pub const COLUMN_3: &str = "3";
    pub const DELIMITER: &str = "output-delimiter";
    pub const DELIMITER_DEFAULT: &str = "\t";
    pub const FILE_1: &str = "FILE1";
    pub const FILE_2: &str = "FILE2";
}

fn get_usage() -> String {
    format!("{} [OPTION]... FILE1 FILE2", executable!())
}

fn mkdelim(col: usize, opts: &ArgMatches) -> String {
    let mut s = String::new();
    let delim = opts.value_of(options::DELIMITER).unwrap();

    if col > 1 && !opts.is_present(options::COLUMN_1) {
        s.push_str(delim.as_ref());
    }
    if col > 2 && !opts.is_present(options::COLUMN_2) {
        s.push_str(delim.as_ref());
    }

    s
}

fn ensure_nl(line: &mut String) {
    match line.chars().last() {
        Some('\n') => (),
        _ => line.push('\n'),
    }
}

enum LineReader {
    Stdin(Stdin),
    FileIn(BufReader<File>),
}

impl LineReader {
    fn read_line(&mut self, buf: &mut String) -> io::Result<usize> {
        match *self {
            LineReader::Stdin(ref mut r) => r.read_line(buf),
            LineReader::FileIn(ref mut r) => r.read_line(buf),
        }
    }
}

fn comm(a: &mut LineReader, b: &mut LineReader, opts: &ArgMatches) {
    let delim: Vec<String> = (0..4).map(|col| mkdelim(col, opts)).collect();

    let ra = &mut String::new();
    let mut na = a.read_line(ra);
    let rb = &mut String::new();
    let mut nb = b.read_line(rb);

    while na.is_ok() || nb.is_ok() {
        let ord = match (na.is_ok(), nb.is_ok()) {
            (false, true) => Ordering::Greater,
            (true, false) => Ordering::Less,
            (true, true) => match (&na, &nb) {
                (&Ok(0), &Ok(0)) => break,
                (&Ok(0), _) => Ordering::Greater,
                (_, &Ok(0)) => Ordering::Less,
                _ => ra.cmp(&rb),
            },
            _ => unreachable!(),
        };

        match ord {
            Ordering::Less => {
                if !opts.is_present(options::COLUMN_1) {
                    ensure_nl(ra);
                    print!("{}{}", delim[1], ra);
                }
                ra.clear();
                na = a.read_line(ra);
            }
            Ordering::Greater => {
                if !opts.is_present(options::COLUMN_2) {
                    ensure_nl(rb);
                    print!("{}{}", delim[2], rb);
                }
                rb.clear();
                nb = b.read_line(rb);
            }
            Ordering::Equal => {
                if !opts.is_present(options::COLUMN_3) {
                    ensure_nl(ra);
                    print!("{}{}", delim[3], ra);
                }
                ra.clear();
                rb.clear();
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
            let f = File::open(&Path::new(name))?;
            Ok(LineReader::FileIn(BufReader::new(f)))
        }
    }
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let usage = get_usage();

    let matches = App::new(executable!())
        .version(VERSION)
        .about(ABOUT)
        .usage(&usage[..])
        .after_help(LONG_HELP)
        .arg(
            Arg::with_name(options::COLUMN_1)
                .short(options::COLUMN_1)
                .help("suppress column 1 (lines unique to FILE1)"),
        )
        .arg(
            Arg::with_name(options::COLUMN_2)
                .short(options::COLUMN_2)
                .help("suppress column 2 (lines unique to FILE2)"),
        )
        .arg(
            Arg::with_name(options::COLUMN_3)
                .short(options::COLUMN_3)
                .help("suppress column 3 (lines that appear in both files)"),
        )
        .arg(
            Arg::with_name(options::DELIMITER)
                .long(options::DELIMITER)
                .help("separate columns with STR")
                .value_name("STR")
                .default_value(options::DELIMITER_DEFAULT)
                .hide_default_value(true),
        )
        .arg(Arg::with_name(options::FILE_1).required(true))
        .arg(Arg::with_name(options::FILE_2).required(true))
        .get_matches_from(args);

    let mut f1 = open_file(matches.value_of(options::FILE_1).unwrap()).unwrap();
    let mut f2 = open_file(matches.value_of(options::FILE_2).unwrap()).unwrap();

    comm(&mut f1, &mut f2, &matches);

    0
}
