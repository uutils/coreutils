// TODO: Make -w flag work with decimals
// TODO: Support -f flag

// spell-checker:ignore (ToDO) istr chiter argptr ilen

#[macro_use]
extern crate uucore;

use clap::{App, AppSettings, Arg};
use num_bigint::BigInt;
use num_traits::One;
use num_traits::Zero;
use num_traits::{Num, ToPrimitive};
use std::cmp;
use std::io::{stdout, Write};
use std::str::FromStr;

static VERSION: &str = env!("CARGO_PKG_VERSION");
static ABOUT: &str = "Display numbers from FIRST to LAST, in steps of INCREMENT.";
static OPT_SEPARATOR: &str = "separator";
static OPT_TERMINATOR: &str = "terminator";
static OPT_WIDTHS: &str = "widths";

static ARG_NUMBERS: &str = "numbers";

fn get_usage() -> String {
    format!(
        "{0} [OPTION]... LAST
    {0} [OPTION]... FIRST LAST
    {0} [OPTION]... FIRST INCREMENT LAST",
        executable!()
    )
}
#[derive(Clone)]
struct SeqOptions {
    separator: String,
    terminator: Option<String>,
    widths: bool,
}

enum Number {
    BigInt(BigInt),
    F64(f64),
}

impl Number {
    fn is_zero(&self) -> bool {
        match self {
            Number::BigInt(n) => n.is_zero(),
            Number::F64(n) => n.is_zero(),
        }
    }

    fn into_f64(self) -> f64 {
        match self {
            // BigInt::to_f64() can not return None.
            Number::BigInt(n) => n.to_f64().unwrap(),
            Number::F64(n) => n,
        }
    }
}

impl FromStr for Number {
    type Err = String;
    /// Tries to parse this string as a BigInt, or if that fails as an f64.
    fn from_str(mut s: &str) -> Result<Self, Self::Err> {
        if s.starts_with('+') {
            s = &s[1..];
        }

        match s.parse::<BigInt>() {
            Ok(n) => Ok(Number::BigInt(n)),
            Err(_) => match s.parse::<f64>() {
                Ok(n) => Ok(Number::F64(n)),
                Err(e) => Err(format!(
                    "seq: invalid floating point argument `{}`: {}",
                    s, e
                )),
            },
        }
    }
}

fn escape_sequences(s: &str) -> String {
    s.replace("\\n", "\n").replace("\\t", "\t")
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let usage = get_usage();
    let matches = App::new(executable!())
        .setting(AppSettings::AllowLeadingHyphen)
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
            Arg::with_name(ARG_NUMBERS)
                .multiple(true)
                .takes_value(true)
                .allow_hyphen_values(true)
                .max_values(3),
        )
        .get_matches_from(args);

    let numbers = matches.values_of(ARG_NUMBERS).unwrap().collect::<Vec<_>>();

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
        let slice = numbers[0];
        let len = slice.len();
        let dec = slice.find('.').unwrap_or(len);
        largest_dec = len - dec;
        padding = dec;
        match slice.parse() {
            Ok(n) => n,
            Err(s) => {
                show_error!("{}", s);
                return 1;
            }
        }
    } else {
        Number::BigInt(BigInt::one())
    };
    let increment = if numbers.len() > 2 {
        let slice = numbers[1];
        let len = slice.len();
        let dec = slice.find('.').unwrap_or(len);
        largest_dec = cmp::max(largest_dec, len - dec);
        padding = cmp::max(padding, dec);
        match slice.parse() {
            Ok(n) => n,
            Err(s) => {
                show_error!("{}", s);
                return 1;
            }
        }
    } else {
        Number::BigInt(BigInt::one())
    };
    if increment.is_zero() {
        show_error!("increment value: '{}'", numbers[1]);
        return 1;
    }
    let last = {
        let slice = numbers[numbers.len() - 1];
        padding = cmp::max(padding, slice.find('.').unwrap_or_else(|| slice.len()));
        match slice.parse::<Number>() {
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

    match (first, last, increment) {
        (Number::BigInt(first), Number::BigInt(last), Number::BigInt(increment)) => {
            print_seq_integers(
                first,
                increment,
                last,
                separator,
                terminator,
                options.widths,
                padding,
            )
        }
        (first, last, increment) => print_seq(
            first.into_f64(),
            increment.into_f64(),
            last.into_f64(),
            largest_dec,
            separator,
            terminator,
            options.widths,
            padding,
        ),
    }
    0
}

fn done_printing<T: Num + PartialOrd>(next: &T, increment: &T, last: &T) -> bool {
    if increment >= &T::zero() {
        next > last
    } else {
        next < last
    }
}

/// Floating point based code path
#[allow(clippy::too_many_arguments)]
fn print_seq(
    first: f64,
    increment: f64,
    last: f64,
    largest_dec: usize,
    separator: String,
    terminator: String,
    pad: bool,
    padding: usize,
) {
    let mut i = 0isize;
    let mut value = first + i as f64 * increment;
    while !done_printing(&value, &increment, &last) {
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
        value = first + i as f64 * increment;
        if !done_printing(&value, &increment, &last) {
            print!("{}", separator);
        }
    }
    if (first >= last && increment < 0f64) || (first <= last && increment > 0f64) {
        print!("{}", terminator);
    }
    crash_if_err!(1, stdout().flush());
}

/// BigInt based code path
fn print_seq_integers(
    first: BigInt,
    increment: BigInt,
    last: BigInt,
    separator: String,
    terminator: String,
    pad: bool,
    padding: usize,
) {
    let mut value = first;
    let mut is_first_iteration = true;
    while !done_printing(&value, &increment, &last) {
        if !is_first_iteration {
            print!("{}", separator);
        }
        is_first_iteration = false;
        if pad {
            print!("{number:>0width$}", number = value, width = padding);
        } else {
            print!("{}", value);
        }
        value += &increment;
    }

    if !is_first_iteration {
        print!("{}", terminator);
    }
}
