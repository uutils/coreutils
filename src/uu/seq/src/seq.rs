// TODO: Make -w flag work with decimals
// TODO: Support -f flag

// spell-checker:ignore (ToDO) istr chiter argptr ilen extendedbigdecimal extendedbigint numberparse

#[macro_use]
extern crate uucore;

use clap::{crate_version, App, AppSettings, Arg};
use num_traits::Zero;
use std::io::{stdout, ErrorKind, Write};

mod extendedbigdecimal;
mod extendedbigint;
mod number;
mod numberparse;
use crate::extendedbigdecimal::ExtendedBigDecimal;
use crate::extendedbigint::ExtendedBigInt;
use crate::number::Number;
use crate::number::PreciseNumber;
use crate::numberparse::ParseNumberError;

use uucore::display::Quotable;

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
type RangeInt = (ExtendedBigInt, ExtendedBigInt, ExtendedBigInt);

/// A range of floats.
///
/// The elements are (first, increment, last).
type RangeFloat = (ExtendedBigDecimal, ExtendedBigDecimal, ExtendedBigDecimal);

/// Terminate the process with error code 1.
///
/// Before terminating the process, this function prints an error
/// message that depends on `arg` and `e`.
///
/// Although the signature of this function states that it returns a
/// [`PreciseNumber`], it never reaches the return statement. It is just
/// there to make it easier to use this function when unwrapping the
/// result of calling [`str::parse`] when attempting to parse a
/// [`PreciseNumber`].
///
/// # Examples
///
/// ```rust,ignore
/// let s = "1.2e-3";
/// s.parse::<PreciseNumber>.unwrap_or_else(|e| exit_with_error(s, e))
/// ```
fn exit_with_error(arg: &str, e: ParseNumberError) -> ! {
    match e {
        ParseNumberError::Float => crash!(
            1,
            "invalid floating point argument: {}\nTry '{} --help' for more information.",
            arg.quote(),
            uucore::execution_phrase()
        ),
        ParseNumberError::Nan => crash!(
            1,
            "invalid 'not-a-number' argument: {}\nTry '{} --help' for more information.",
            arg.quote(),
            uucore::execution_phrase()
        ),
        ParseNumberError::Hex => crash!(
            1,
            "invalid hexadecimal argument: {}\nTry '{} --help' for more information.",
            arg.quote(),
            uucore::execution_phrase()
        ),
    }
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let usage = usage();
    let matches = uu_app().usage(&usage[..]).get_matches_from(args);

    let numbers = matches.values_of(ARG_NUMBERS).unwrap().collect::<Vec<_>>();

    let options = SeqOptions {
        separator: matches.value_of(OPT_SEPARATOR).unwrap_or("\n").to_string(),
        terminator: matches.value_of(OPT_TERMINATOR).unwrap_or("\n").to_string(),
        widths: matches.is_present(OPT_WIDTHS),
    };

    let first = if numbers.len() > 1 {
        let slice = numbers[0];
        slice.parse().unwrap_or_else(|e| exit_with_error(slice, e))
    } else {
        PreciseNumber::one()
    };
    let increment = if numbers.len() > 2 {
        let slice = numbers[1];
        slice.parse().unwrap_or_else(|e| exit_with_error(slice, e))
    } else {
        PreciseNumber::one()
    };
    if increment.is_zero() {
        show_error!(
            "invalid Zero increment value: '{}'\nTry '{} --help' for more information.",
            numbers[1],
            uucore::execution_phrase()
        );
        return 1;
    }
    let last: PreciseNumber = {
        let slice = numbers[numbers.len() - 1];
        slice.parse().unwrap_or_else(|e| exit_with_error(slice, e))
    };

    let padding = first
        .num_integral_digits
        .max(increment.num_integral_digits)
        .max(last.num_integral_digits);
    let largest_dec = first
        .num_fractional_digits
        .max(increment.num_fractional_digits);

    let result = match (first.number, increment.number, last.number) {
        (Number::Int(first), Number::Int(increment), last) => print_seq_integers(
            (first, increment, last.into_extended_big_int()),
            options.separator,
            options.terminator,
            options.widths,
            padding,
        ),
        (first, increment, last) => print_seq(
            (
                first.into_extended_big_decimal(),
                increment.into_extended_big_decimal(),
                last.into_extended_big_decimal(),
            ),
            largest_dec,
            options.separator,
            options.terminator,
            options.widths,
            padding,
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

fn done_printing<T: Zero + PartialOrd>(next: &T, increment: &T, last: &T) -> bool {
    if increment >= &T::zero() {
        next > last
    } else {
        next < last
    }
}

/// Write a big decimal formatted according to the given parameters.
///
/// This method is an adapter to support displaying negative zero on
/// Rust versions earlier than 1.53.0. After that version, we should be
/// able to display negative zero using the default formatting provided
/// by `-0.0f32`, for example.
fn write_value_float(
    writer: &mut impl Write,
    value: &ExtendedBigDecimal,
    width: usize,
    precision: usize,
    is_first_iteration: bool,
) -> std::io::Result<()> {
    let value_as_str = {
        let s = if *value == ExtendedBigDecimal::MinusZero && is_first_iteration {
            format!(
                "-{value:>0width$.precision$}",
                value = value,
                width = if width > 0 { width - 1 } else { width },
                precision = precision,
            )
        } else {
            format!(
                "{value:>0width$.precision$}",
                value = value,
                width = width,
                precision = precision,
            )
        };
        if *value == ExtendedBigDecimal::MinusZero && !s.starts_with('-') {
            [String::from("-"), s].concat()
        } else {
            s
        }
    };
    write!(writer, "{}", value_as_str)
}

/// Write a big int formatted according to the given parameters.
fn write_value_int(
    writer: &mut impl Write,
    value: &ExtendedBigInt,
    width: usize,
    pad: bool,
    is_first_iteration: bool,
) -> std::io::Result<()> {
    let value_as_str = if pad {
        let s = if *value == ExtendedBigInt::MinusZero && is_first_iteration {
            format!("-{value:>0width$}", value = value, width = width - 1,)
        } else {
            format!("{value:>0width$}", value = value, width = width,)
        };
        if *value == ExtendedBigInt::MinusZero && !s.starts_with('-') {
            [String::from("-"), s].concat()
        } else {
            s
        }
    } else if *value == ExtendedBigInt::MinusZero && is_first_iteration {
        format!("-{}", value)
    } else {
        format!("{}", value)
    };
    write!(writer, "{}", value_as_str)
}

// TODO `print_seq()` and `print_seq_integers()` are nearly identical,
// they could be refactored into a single more general function.

/// Floating point based code path
fn print_seq(
    range: RangeFloat,
    largest_dec: usize,
    separator: String,
    terminator: String,
    pad: bool,
    padding: usize,
) -> std::io::Result<()> {
    let stdout = stdout();
    let mut stdout = stdout.lock();
    let (first, increment, last) = range;
    let mut value = first;
    let padding = if pad { padding + 1 + largest_dec } else { 0 };
    let mut is_first_iteration = true;
    while !done_printing(&value, &increment, &last) {
        if !is_first_iteration {
            write!(stdout, "{}", separator)?;
        }
        write_value_float(
            &mut stdout,
            &value,
            padding,
            largest_dec,
            is_first_iteration,
        )?;
        // TODO Implement augmenting addition.
        value = value + increment.clone();
        is_first_iteration = false;
    }
    if !is_first_iteration {
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
        write_value_int(&mut stdout, &value, padding, pad, is_first_iteration)?;
        // TODO Implement augmenting addition.
        value = value + increment.clone();
        is_first_iteration = false;
    }

    if !is_first_iteration {
        write!(stdout, "{}", terminator)?;
    }
    Ok(())
}
