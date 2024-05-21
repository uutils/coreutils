// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore (ToDO) extendedbigdecimal numberparse
use std::io::{stdout, ErrorKind, Write};

use clap::{crate_version, Arg, ArgAction, Command};
use num_traits::{ToPrimitive, Zero};

use uucore::error::{FromIo, UResult};
use uucore::format::{num_format, Format};
use uucore::{format_usage, help_about, help_usage};

mod error;
mod extendedbigdecimal;
// public to allow fuzzing
#[cfg(fuzzing)]
pub mod number;
#[cfg(not(fuzzing))]
mod number;
mod numberparse;
use crate::error::SeqError;
use crate::extendedbigdecimal::ExtendedBigDecimal;
use crate::number::PreciseNumber;

const ABOUT: &str = help_about!("seq.md");
const USAGE: &str = help_usage!("seq.md");

const OPT_SEPARATOR: &str = "separator";
const OPT_TERMINATOR: &str = "terminator";
const OPT_EQUAL_WIDTH: &str = "equal-width";
const OPT_FORMAT: &str = "format";

const ARG_NUMBERS: &str = "numbers";

#[derive(Clone)]
struct SeqOptions<'a> {
    separator: String,
    terminator: String,
    equal_width: bool,
    format: Option<&'a str>,
}

/// A range of floats.
///
/// The elements are (first, increment, last).
type RangeFloat = (ExtendedBigDecimal, ExtendedBigDecimal, ExtendedBigDecimal);

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().try_get_matches_from(args)?;

    let numbers_option = matches.get_many::<String>(ARG_NUMBERS);

    if numbers_option.is_none() {
        return Err(SeqError::NoArguments.into());
    }

    let numbers = numbers_option.unwrap().collect::<Vec<_>>();

    let options = SeqOptions {
        separator: matches
            .get_one::<String>(OPT_SEPARATOR)
            .map(|s| s.as_str())
            .unwrap_or("\n")
            .to_string(),
        terminator: matches
            .get_one::<String>(OPT_TERMINATOR)
            .map(|s| s.as_str())
            .unwrap_or("\n")
            .to_string(),
        equal_width: matches.get_flag(OPT_EQUAL_WIDTH),
        format: matches.get_one::<String>(OPT_FORMAT).map(|s| s.as_str()),
    };

    let first = if numbers.len() > 1 {
        match numbers[0].parse() {
            Ok(num) => num,
            Err(e) => return Err(SeqError::ParseError(numbers[0].to_string(), e).into()),
        }
    } else {
        PreciseNumber::one()
    };
    let increment = if numbers.len() > 2 {
        match numbers[1].parse() {
            Ok(num) => num,
            Err(e) => return Err(SeqError::ParseError(numbers[1].to_string(), e).into()),
        }
    } else {
        PreciseNumber::one()
    };
    if increment.is_zero() {
        return Err(SeqError::ZeroIncrement(numbers[1].to_string()).into());
    }
    let last: PreciseNumber = {
        // We are guaranteed that `numbers.len()` is greater than zero
        // and at most three because of the argument specification in
        // `uu_app()`.
        let n: usize = numbers.len();
        match numbers[n - 1].parse() {
            Ok(num) => num,
            Err(e) => return Err(SeqError::ParseError(numbers[n - 1].to_string(), e).into()),
        }
    };

    let padding = first
        .num_integral_digits
        .max(increment.num_integral_digits)
        .max(last.num_integral_digits);
    let largest_dec = first
        .num_fractional_digits
        .max(increment.num_fractional_digits);

    let format = match options.format {
        Some(f) => {
            let f = Format::<num_format::Float>::parse(f)?;
            Some(f)
        }
        None => None,
    };
    let result = print_seq(
        (first.number, increment.number, last.number),
        largest_dec,
        &options.separator,
        &options.terminator,
        options.equal_width,
        padding,
        &format,
    );
    match result {
        Ok(_) => Ok(()),
        Err(err) if err.kind() == ErrorKind::BrokenPipe => Ok(()),
        Err(e) => Err(e.map_err_context(|| "write error".into())),
    }
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .trailing_var_arg(true)
        .allow_negative_numbers(true)
        .infer_long_args(true)
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .arg(
            Arg::new(OPT_SEPARATOR)
                .short('s')
                .long("separator")
                .help("Separator character (defaults to \\n)"),
        )
        .arg(
            Arg::new(OPT_TERMINATOR)
                .short('t')
                .long("terminator")
                .help("Terminator character (defaults to \\n)"),
        )
        .arg(
            Arg::new(OPT_EQUAL_WIDTH)
                .short('w')
                .long("equal-width")
                .help("Equalize widths of all numbers by padding with zeros")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_FORMAT)
                .short('f')
                .long(OPT_FORMAT)
                .help("use printf style floating-point FORMAT"),
        )
        .arg(
            Arg::new(ARG_NUMBERS)
                .action(ArgAction::Append)
                .num_args(1..=3),
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
fn write_value_float(
    writer: &mut impl Write,
    value: &ExtendedBigDecimal,
    width: usize,
    precision: usize,
) -> std::io::Result<()> {
    let value_as_str =
        if *value == ExtendedBigDecimal::Infinity || *value == ExtendedBigDecimal::MinusInfinity {
            format!("{value:>width$.precision$}")
        } else {
            format!("{value:>0width$.precision$}")
        };
    write!(writer, "{value_as_str}")
}

/// Floating point based code path
fn print_seq(
    range: RangeFloat,
    largest_dec: usize,
    separator: &str,
    terminator: &str,
    pad: bool,
    padding: usize,
    format: &Option<Format<num_format::Float>>,
) -> std::io::Result<()> {
    let stdout = stdout();
    let mut stdout = stdout.lock();
    let (first, increment, last) = range;
    let mut value = first;
    let padding = if pad {
        padding + if largest_dec > 0 { largest_dec + 1 } else { 0 }
    } else {
        0
    };
    let mut is_first_iteration = true;
    while !done_printing(&value, &increment, &last) {
        if !is_first_iteration {
            write!(stdout, "{separator}")?;
        }
        // If there was an argument `-f FORMAT`, then use that format
        // template instead of the default formatting strategy.
        //
        // TODO The `printf()` method takes a string as its second
        // parameter but we have an `ExtendedBigDecimal`. In order to
        // satisfy the signature of the function, we convert the
        // `ExtendedBigDecimal` into a string. The `printf()`
        // logic will subsequently parse that string into something
        // similar to an `ExtendedBigDecimal` again before rendering
        // it as a string and ultimately writing to `stdout`. We
        // shouldn't have to do so much converting back and forth via
        // strings.
        match &format {
            Some(f) => {
                let float = match &value {
                    ExtendedBigDecimal::BigDecimal(bd) => bd.to_f64().unwrap(),
                    ExtendedBigDecimal::Infinity => f64::INFINITY,
                    ExtendedBigDecimal::MinusInfinity => f64::NEG_INFINITY,
                    ExtendedBigDecimal::MinusZero => -0.0,
                    ExtendedBigDecimal::Nan => f64::NAN,
                };
                f.fmt(&mut stdout, float)?;
            }
            None => write_value_float(&mut stdout, &value, padding, largest_dec)?,
        }
        // TODO Implement augmenting addition.
        value = value + increment.clone();
        is_first_iteration = false;
    }
    if !is_first_iteration {
        write!(stdout, "{terminator}")?;
    }
    stdout.flush()?;
    Ok(())
}
