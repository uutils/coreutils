#![crate_name = "uu_sort"]

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

use std::cmp::Ordering;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Read, stdin, stdout, Write};
use std::path::Path;
use uucore::fs::is_stdin_interactive;

static NAME: &'static str = "sort";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

static DECIMAL_PT: char = '.';
static THOUSANDS_SEP: char = ',';

enum SortMode {
    Numeric,
    HumanNumeric,
    Month,
    Default,
}

struct Settings {
    mode: SortMode,
    reverse: bool,
    outfile: Option<String>,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            mode: SortMode::Default,
            reverse: false,
            outfile: None,
        }
    }
}

pub fn uumain(args: Vec<String>) -> i32 {
    let mut settings: Settings = Default::default();
    let mut opts = getopts::Options::new();

    opts.optflag("n", "numeric-sort", "compare according to string numerical value");
    opts.optflag("h", "human-numeric-sort", "compare according to human readable sizes, eg 1M > 100k");
    opts.optflag("M", "month-sort", "compare according to month name abbreviation");
    opts.optflag("r", "reverse", "reverse the output");
    opts.optflag("h", "help", "display this help and exit");
    opts.optflag("", "version", "output version information and exit");
    opts.optopt("o", "output", "write output to FILENAME instead of stdout", "FILENAME");

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

    settings.mode = if matches.opt_present("numeric-sort") {
        SortMode::Numeric
    } else if matches.opt_present("human-numeric-sort") {
        SortMode::HumanNumeric
    } else if matches.opt_present("month-sort") {
        SortMode::Month
    } else {
        SortMode::Default
    };

    settings.reverse = matches.opt_present("reverse");
    settings.outfile = matches.opt_str("output");

    let mut files = matches.free;
    if files.is_empty() {
        /* if no file, default to stdin */
        files.push("-".to_owned());
    }

    exec(files, &settings);

    0
}

fn exec(files: Vec<String>, settings: &Settings) {
    for path in &files {
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

        match settings.mode {
            SortMode::Numeric => lines.sort_by(numeric_compare),
            SortMode::HumanNumeric => lines.sort_by(human_numeric_size_compare),
            SortMode::Month => lines.sort_by(month_compare),
            SortMode::Default => lines.sort()
        }

        let iter = lines.iter();
        if settings.reverse {
            print_sorted(iter.rev(), &settings.outfile);
        } else {
            print_sorted(iter, &settings.outfile)
        };
    }
}

/// Parse the beginning string into an f64, returning -inf instead of NaN on errors.
fn permissive_f64_parse(a: &str) -> f64 {
    // Maybe should be split on non-digit, but then 10e100 won't parse properly.
    // On the flip side, this will give NEG_INFINITY for "1,234", which might be OK
    // because there's no way to handle both CSV and thousands separators without a new flag.
    // GNU sort treats "1,234" as "1" in numeric, so maybe it's fine.
    let sa: &str = a.split_whitespace().next().unwrap();
    match sa.parse::<f64>() {
        Ok(a) => a,
        Err(_) => std::f64::NEG_INFINITY
    }
}

/// Compares two floating point numbers, with errors being assumed to be -inf.
/// Stops coercing at the first whitespace char, so 1e2 will parse as 100 but 
/// 1,000 will parse as -inf.
fn numeric_compare(a: &String, b: &String) -> Ordering {
    let fa = permissive_f64_parse(a);
    let fb = permissive_f64_parse(b);
    // f64::cmp isn't implemented because NaN messes with it
    // but we sidestep that with permissive_f64_parse so just fake it
    if fa > fb {
        Ordering::Greater
    }
    else if fa < fb {
        Ordering::Less
    }
    else {
        Ordering::Equal
    }
}

fn human_numeric_convert(a: &String) -> f64 {
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
    int_part * suffix
}

/// Compare two strings as if they are human readable sizes.
/// AKA 1M > 100k
fn human_numeric_size_compare(a: &String, b: &String) -> Ordering {
    let fa = human_numeric_convert(a);
    let fb = human_numeric_convert(b);
    if fa > fb {
        Ordering::Greater
    }
    else if fa < fb {
        Ordering::Less
    }
    else {
        Ordering::Equal
    }
}

#[derive(Eq, Ord, PartialEq, PartialOrd)]
enum Month {
    Unknown     = 0,
    January     = 1,
    February    = 2,
    March       = 3,
    April       = 4,
    May         = 5,
    June        = 6,
    July        = 7,
    August      = 8,
    September   = 9,
    October     = 10,
    November    = 11,
    December    = 12,
}

/// Parse the beginning string into a Month, returning Month::Unknown on errors.
fn month_parse(line: &String) -> Month {
    match line.split_whitespace().next().unwrap().to_uppercase().as_ref() {
        "JAN" => Month::January,
        "FEB" => Month::February,
        "MAR" => Month::March,
        "APR" => Month::April,
        "MAY" => Month::May,
        "JUN" => Month::June,
        "JUL" => Month::July,
        "AUG" => Month::August,
        "SEP" => Month::September,
        "OCT" => Month::October,
        "NOV" => Month::November,
        "DEC" => Month::December,
        _     => Month::Unknown,
    }
}

fn month_compare(a: &String, b: &String) -> Ordering {
    month_parse(a).cmp(&month_parse(b))
}

fn print_sorted<S, T: Iterator<Item=S>>(iter: T, outfile: &Option<String>) where S: std::fmt::Display {
    let mut file: Box<Write> = match *outfile {
        Some(ref filename) => {
            match File::create(Path::new(&filename)) {
                Ok(f) => Box::new(BufWriter::new(f)) as Box<Write>,
                Err(e) => {
                    show_error!("sort: {0}: {1}", filename, e.to_string());
                    panic!("Could not open output file");
                },
            }
        },
        None => Box::new(stdout()) as Box<Write>,
    };


    for line in iter {
        let str = format!("{}\n", line);
        match file.write_all(str.as_bytes()) {
            Err(e) => {
                show_error!("sort: {0}", e.to_string());
                panic!("Write failed");
            },
            Ok(_) => (),
        }
    }
}

// from cat.rs
fn open(path: &str) -> Option<(Box<Read>, bool)> {
    if path == "-" {
        let stdin = stdin();
        return Some((Box::new(stdin) as Box<Read>, is_stdin_interactive()));
    }

    match File::open(Path::new(path)) {
        Ok(f) => Some((Box::new(f) as Box<Read>, false)),
        Err(e) => {
            show_error!("sort: {0}: {1}", path, e.to_string());
            None
        },
    }
}
