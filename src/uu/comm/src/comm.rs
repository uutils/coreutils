// This file is part of the uutils coreutils package.
//
// (c) Michael Gehring <mg@ebfe.org>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) delim mkdelim

#[macro_use]
extern crate uucore;

use clap::{App, Arg};
use std::cmp::Ordering;
use std::fs::File;
use std::io::{self, stdin, BufRead, BufReader, Stdin};
use std::path::Path;

static USAGE: &str = "[OPTIONS] FILE1 FILE2";
static SUMMARY: &str = "Compare sorted files line by line";

fn mkdelim(col: usize, opts: &clap::ArgMatches) -> String {
    let mut s = String::new();
    let delim = match opts.value_of("output-delimiter") {
        Some(d) => d.to_owned(),
        None => "\t".to_owned(),
    };

    if col > 1 && !opts.is_present("1") {
        s.push_str(delim.as_ref());
    }
    if col > 2 && !opts.is_present("2") {
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

fn comm(a: &mut LineReader, b: &mut LineReader, opts: &clap::ArgMatches) {
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
                if !opts.is_present("1") {
                    ensure_nl(ra);
                    print!("{}{}", delim[1], ra);
                }
                ra.clear();
                na = a.read_line(ra);
            }
            Ordering::Greater => {
                if !opts.is_present("2") {
                    ensure_nl(rb);
                    print!("{}{}", delim[2], rb);
                }
                rb.clear();
                nb = b.read_line(rb);
            }
            Ordering::Equal => {
                if !opts.is_present("3") {
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
    let args = args.collect_str();

    let matches = App::new(executable!())
        .about(SUMMARY)
        .usage(USAGE)
        .arg(
            Arg::with_name("1")
                .short("1")
                .help("suppress column 1 (lines uniq to FILE1)"),
        )
        .arg(
            Arg::with_name("2")
                .short("2")
                .help("suppress column 2 (lines uniq to FILE2)"),
        )
        .arg(
            Arg::with_name("3")
                .short("3")
                .help("suppress column 3 (lines that appear in both files)"),
        )
        .arg(
            Arg::with_name("output-delimiter")
                .long("output-delimiter")
                .value_name("STR")
                .help("separate columns with STR"),
        )
        .arg(Arg::with_name("FILE1").index(1))
        .arg(Arg::with_name("FILE2").index(2))
        .get_matches_from(args);

    let mut f1 = open_file(matches.value_of("FILE1").unwrap_or("-").as_ref()).unwrap();
    let mut f2 = open_file(matches.value_of("FILE2").unwrap_or("-").as_ref()).unwrap();

    comm(&mut f1, &mut f2, &matches);

    0
}
