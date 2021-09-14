// TODO: Make -w flag work with decimals
// TODO: Support -f flag

// spell-checker:ignore (ToDO) istr chiter argptr ilen bigdecimal

#[macro_use]
extern crate uucore;

use bigdecimal::BigDecimal;
use clap::{crate_version, App, AppSettings, Arg};
use num_bigint::BigInt;
use num_traits::Num;
use num_traits::One;
use num_traits::Zero;
use std::cmp;
use std::io::{stdout, ErrorKind, Write};

mod number;
use crate::number::Number;

static ABOUT: &str = "Display numbers from FIRST to LAST, in steps of INCREMENT.";
static OPT_SEPARATOR: &str = "separator";
static OPT_TERMINATOR: &str = "terminator";
static OPT_WIDTHS: &str = "widths";

static ARG_NUMBERS: &str = "numbers";

fn usage() -> String {
    format!(
        "{0} [OPTION]... LAST
    {0} [OPTION]... FIRST LAST
    {0} [OPTION]... FIRST INCREMENT LAST",
        uucore::execution_phrase()
    )
}
#[derive(Clone)]
struct SeqOptions {
    separator: String,
    terminator: String,
    widths: bool,
}

/// A range of integers.
///
/// The elements are (first, increment, last).
type RangeInt = (BigInt, BigInt, BigInt);

/// A range of `BigDecimal` numbers.
///
/// The elements are (first, increment, last).
type RangeBigDecimal = (BigDecimal, BigDecimal, BigDecimal);

pub fn uumain(args: impl uucore::Args) -> i32 {
    let usage = usage();
    let matches = uu_app().usage(&usage[..]).get_matches_from(args);

    let numbers = matches.values_of(ARG_NUMBERS).unwrap().collect::<Vec<_>>();

    let options = SeqOptions {
        separator: matches.value_of(OPT_SEPARATOR).unwrap_or("\n").to_string(),
        terminator: matches.value_of(OPT_TERMINATOR).unwrap_or("\n").to_string(),
        widths: matches.is_present(OPT_WIDTHS),
    };

    let mut largest_dec = 0;
    let first = if numbers.len() > 1 {
        let slice = numbers[0];
        let len = slice.len();
        let dec = slice.find('.').unwrap_or(len);
        largest_dec = len - dec;
        crash_if_err!(1, slice.parse())
    } else {
        Number::BigInt(BigInt::one())
    };
    let increment = if numbers.len() > 2 {
        let slice = numbers[1];
        let len = slice.len();
        let dec = slice.find('.').unwrap_or(len);
        largest_dec = cmp::max(largest_dec, len - dec);
        crash_if_err!(1, slice.parse())
    } else {
        Number::BigInt(BigInt::one())
    };
    if increment.is_zero() {
        show_error!(
            "invalid Zero increment value: '{}'\nTry '{} --help' for more information.",
            numbers[1],
            uucore::execution_phrase()
        );
        return 1;
    }
    let last: Number = {
        let slice = numbers[numbers.len() - 1];
        crash_if_err!(1, slice.parse())
    };
    if largest_dec > 0 {
        largest_dec -= 1;
    }
    let padding = first
        .num_digits()
        .max(increment.num_digits())
        .max(last.num_digits());
    let result = match (first, last, increment) {
        (Number::MinusZeroInt, Number::BigInt(last), Number::BigInt(increment)) => {
            print_seq_integers(
                (BigInt::zero(), increment, last),
                options.separator,
                options.terminator,
                options.widths,
                padding,
                true,
            )
        }
        (Number::BigInt(first), Number::BigInt(last), Number::BigInt(increment)) => {
            print_seq_integers(
                (first, increment, last),
                options.separator,
                options.terminator,
                options.widths,
                padding,
                false,
            )
        }
        (Number::MinusZeroFloat | Number::MinusZeroInt, last, increment) => print_seq(
            (
                BigDecimal::zero(),
                increment.into_big_decimal(),
                last.into_big_decimal(),
            ),
            largest_dec,
            options.separator,
            options.terminator,
            options.widths,
            padding,
            true,
        ),
        (first, last, increment) => print_seq(
            (
                first.into_big_decimal(),
                increment.into_big_decimal(),
                last.into_big_decimal(),
            ),
            largest_dec,
            options.separator,
            options.terminator,
            options.widths,
            padding,
            false,
        ),
    };
    match result {
        Ok(_) => 0,
        Err(err) if err.kind() == ErrorKind::BrokenPipe => 0,
        Err(_) => 1,
    }
}

pub fn uu_app() -> App<'static, 'static> {
    App::new(uucore::util_name())
        .setting(AppSettings::AllowLeadingHyphen)
        .version(crate_version!())
        .about(ABOUT)
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
                .help("Terminator character (defaults to \\n)")
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
                .max_values(3)
                .required(true),
        )
}

fn done_printing<T: Num + PartialOrd>(next: &T, increment: &T, last: &T) -> bool {
    if increment >= &T::zero() {
        next > last
    } else {
        next < last
    }
}

/// Print a sequence of floating point numbers.
///
/// This function prints a sequence of floating point numbers. The
/// first, last, and increment are given in the triple `range`. Each is
/// a [`BigDecimal`]. The `separator` is printed between the elements of
/// the sequence and `terminator` after the last element.
///
/// `largest_dec` gives the number of digits of precision to use when
/// printing the digits after the decimal point, padded with zeros on
/// the right. If `pad` is true, then `padding` is used to determine the
/// number of digits to print before the decimal point, padded with
/// zeros on the left.
///
/// If `is_first_minus_zero` is `true`, then the `first` parameter is
/// printed as if it were negative zero. Only set this to `true` if
/// `first` is actually zero.
fn print_seq(
    range: RangeBigDecimal,
    largest_dec: usize,
    separator: String,
    terminator: String,
    pad: bool,
    padding: usize,
    is_first_minus_zero: bool,
) -> std::io::Result<()> {
    let stdout = stdout();
    let mut stdout = stdout.lock();
    let (first, increment, last) = range;
    let mut value = first.clone();
    let mut is_first_iteration = true;
    while !done_printing(&value, &increment, &last) {
        let mut width = padding;
        if is_first_iteration && is_first_minus_zero {
            write!(stdout, "-")?;
            width -= 1;
        }
        is_first_iteration = false;
        let istr = format!("{:.*}", largest_dec, value);
        let ilen = istr.len();
        let before_dec = istr.find('.').unwrap_or(ilen);
        if pad && before_dec < width {
            for _ in 0..(width - before_dec) {
                write!(stdout, "0")?;
            }
        }
        write!(stdout, "{}", istr)?;
        value += &increment;
        if !done_printing(&value, &increment, &last) {
            write!(stdout, "{}", separator)?;
        }
    }
    if (first >= last && increment < BigDecimal::zero())
        || (first <= last && increment > BigDecimal::zero())
    {
        write!(stdout, "{}", terminator)?;
    }
    stdout.flush()?;
    Ok(())
}

/// Print an integer sequence.
///
/// This function prints a sequence of integers defined by `range`,
/// which defines the first integer, last integer, and increment of the
/// range. The `separator` is inserted between each integer and
/// `terminator` is inserted at the end.
///
/// The `pad` parameter indicates whether to pad numbers to the width
/// given in `padding`.
///
/// If `is_first_minus_zero` is `true`, then the `first` parameter is
/// printed as if it were negative zero, even though no such number
/// exists as an integer (negative zero only exists for floating point
/// numbers). Only set this to `true` if `first` is actually zero.
fn print_seq_integers(
    range: RangeInt,
    separator: String,
    terminator: String,
    pad: bool,
    padding: usize,
    is_first_minus_zero: bool,
) -> std::io::Result<()> {
    let stdout = stdout();
    let mut stdout = stdout.lock();
    let (first, increment, last) = range;
    let mut value = first;
    let mut is_first_iteration = true;
    while !done_printing(&value, &increment, &last) {
        if !is_first_iteration {
            write!(stdout, "{}", separator)?;
        }
        let mut width = padding;
        if is_first_iteration && is_first_minus_zero {
            write!(stdout, "-")?;
            width -= 1;
        }
        is_first_iteration = false;
        if pad {
            write!(stdout, "{number:>0width$}", number = value, width = width)?;
        } else {
            write!(stdout, "{}", value)?;
        }
        value += &increment;
    }

    if !is_first_iteration {
        write!(stdout, "{}", terminator)?;
    }
    Ok(())
}
