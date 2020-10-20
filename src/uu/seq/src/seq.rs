// TODO: Make -w flag work with decimals
// TODO: Support -f flag

// spell-checker:ignore (ToDO) istr chiter argptr ilen

extern crate clap;

#[macro_use]
extern crate uucore;

use clap::{App, Arg};
use std::cmp;
use std::io::{stdout, Write};

static VERSION: &str = env!("CARGO_PKG_VERSION");
static ABOUT: &str = "Print sequences of numbers";
static OPT_SEPARATOR: &str = "separator";
static OPT_TERMINATOR: &str = "terminator";
static OPT_WIDTHS: &str = "widths";
static OPT_NUMBERS: &str = "numbers";

fn get_usage() -> String {
    format!("{0} [OPTION]... [FILE]...", executable!())
}
#[derive(Clone)]
struct SeqOptions {
    separator: String,
    terminator: Option<String>,
    widths: bool,
}

fn parse_float(mut s: &str) -> Result<f64, String> {
    if s.starts_with('+') {
        s = &s[1..];
    }
    match s.parse() {
        Ok(n) => Ok(n),
        Err(e) => Err(format!(
            "seq: invalid floating point argument `{}`: {}",
            s, e
        )),
    }
}

fn escape_sequences(s: &str) -> String {
    s.replace("\\n", "\n").replace("\\t", "\t")
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let usage = get_usage();
    let matches = App::new(executable!())
        .version(VERSION)
        .about(ABOUT)
        .usage(&usage[..])
        .arg(
            Arg::with_name(OPT_SEPARATOR)
                .short("s")
                .long("separator")
                .help("Separator character (defaults to \\n)")
                .takes_value(true)
                .number_of_values(1),
        )
        .arg(
            Arg::with_name(OPT_TERMINATOR)
                .short("t")
                .long("terminator")
                .help("Terminator character (defaults to separator)")
                .takes_value(true)
                .number_of_values(1),
        )
        .arg(
            Arg::with_name(OPT_WIDTHS)
                .short("w")
                .long("widths")
                .help("Equalize widths of all numbers by padding with zeros"),
        )
        .arg(
            Arg::with_name(OPT_NUMBERS)
                .multiple(true)
                .takes_value(true)
                .max_values(3),
        )
        .get_matches_from(args);

    let numbers = matches.values_of(OPT_NUMBERS).unwrap().collect::<Vec<_>>();

    let mut options = SeqOptions {
        separator: "\n".to_owned(),
        terminator: None,
        widths: false,
    };
    options.separator = matches.value_of(OPT_SEPARATOR).unwrap_or("\n").to_string();
    options.terminator = matches.value_of(OPT_TERMINATOR).map(String::from);
    options.widths = matches.is_present(OPT_WIDTHS);

    let mut largest_dec = 0;
    let mut padding = 0;
    let first = if numbers.len() > 1 {
        let slice = &numbers[0][..];
        let len = slice.len();
        let dec = slice.find('.').unwrap_or(len);
        largest_dec = len - dec;
        padding = dec;
        match parse_float(slice) {
            Ok(n) => n,
            Err(s) => {
                show_error!("{}", s);
                return 1;
            }
        }
    } else {
        1.0
    };
    let step = if numbers.len() > 2 {
        let slice = &numbers[1][..];
        let len = slice.len();
        let dec = slice.find('.').unwrap_or(len);
        largest_dec = cmp::max(largest_dec, len - dec);
        padding = cmp::max(padding, dec);
        match parse_float(slice) {
            Ok(n) => n,
            Err(s) => {
                show_error!("{}", s);
                return 1;
            }
        }
    } else {
        1.0
    };
    let last = {
        let slice = &numbers[numbers.len() - 1][..];
        padding = cmp::max(padding, slice.find('.').unwrap_or_else(|| slice.len()));
        match parse_float(slice) {
            Ok(n) => n,
            Err(s) => {
                show_error!("{}", s);
                return 1;
            }
        }
    };
    if largest_dec > 0 {
        largest_dec -= 1;
    }
    let separator = escape_sequences(&options.separator[..]);
    let terminator = match options.terminator {
        Some(term) => escape_sequences(&term[..]),
        None => separator.clone(),
    };
    print_seq(
        first,
        step,
        last,
        largest_dec,
        separator,
        terminator,
        options.widths,
        padding,
    );

    0
}

fn done_printing(next: f64, step: f64, last: f64) -> bool {
    if step >= 0f64 {
        next > last
    } else {
        next < last
    }
}

#[allow(clippy::too_many_arguments)]
fn print_seq(
    first: f64,
    step: f64,
    last: f64,
    largest_dec: usize,
    separator: String,
    terminator: String,
    pad: bool,
    padding: usize,
) {
    let mut i = 0isize;
    let mut value = first + i as f64 * step;
    while !done_printing(value, step, last) {
        let istr = format!("{:.*}", largest_dec, value);
        let ilen = istr.len();
        let before_dec = istr.find('.').unwrap_or(ilen);
        if pad && before_dec < padding {
            for _ in 0..(padding - before_dec) {
                print!("0");
            }
        }
        print!("{}", istr);
        i += 1;
        value = first + i as f64 * step;
        if !done_printing(value, step, last) {
            print!("{}", separator);
        }
    }
    if (first >= last && step < 0f64) || (first <= last && step > 0f64) {
        print!("{}", terminator);
    }
    crash_if_err!(1, stdout().flush());
}
