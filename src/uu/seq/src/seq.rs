// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore (ToDO) bigdecimal extendedbigdecimal numberparse hexadecimalfloat
use std::ffi::OsString;
use std::io::{BufWriter, ErrorKind, Write, stdout};

use clap::{Arg, ArgAction, Command};
use num_traits::Zero;

use uucore::error::{FromIo, UResult};
use uucore::format::num_format::FloatVariant;
use uucore::format::{ExtendedBigDecimal, Format, num_format};
use uucore::{format_usage, help_about, help_usage};

mod error;

// public to allow fuzzing
#[cfg(fuzzing)]
pub mod number;
#[cfg(not(fuzzing))]
mod number;
mod numberparse;
use crate::error::SeqError;
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

// Turn short args with attached value, for example "-s,", into two args "-s" and "," to make
// them work with clap.
fn split_short_args_with_value(args: impl uucore::Args) -> impl uucore::Args {
    let mut v: Vec<OsString> = Vec::new();

    for arg in args {
        let bytes = arg.as_encoded_bytes();

        if bytes.len() > 2
            && (bytes.starts_with(b"-f") || bytes.starts_with(b"-s") || bytes.starts_with(b"-t"))
        {
            let (short_arg, value) = bytes.split_at(2);
            // SAFETY:
            // Both `short_arg` and `value` only contain content that originated from `OsStr::as_encoded_bytes`
            v.push(unsafe { OsString::from_encoded_bytes_unchecked(short_arg.to_vec()) });
            v.push(unsafe { OsString::from_encoded_bytes_unchecked(value.to_vec()) });
        } else {
            v.push(arg);
        }
    }

    v.into_iter()
}

fn select_precision(
    first: &PreciseNumber,
    increment: &PreciseNumber,
    last: &PreciseNumber,
) -> Option<usize> {
    match (
        first.num_fractional_digits,
        increment.num_fractional_digits,
        last.num_fractional_digits,
    ) {
        (Some(0), Some(0), Some(0)) => Some(0),
        (Some(f), Some(i), Some(_)) => Some(f.max(i)),
        _ => None,
    }
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().try_get_matches_from(split_short_args_with_value(args))?;

    let numbers_option = matches.get_many::<String>(ARG_NUMBERS);

    if numbers_option.is_none() {
        return Err(SeqError::NoArguments.into());
    }

    let numbers = numbers_option.unwrap().collect::<Vec<_>>();

    let options = SeqOptions {
        separator: matches
            .get_one::<String>(OPT_SEPARATOR)
            .map_or("\n", |s| s.as_str())
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

    let precision = select_precision(&first, &increment, &last);

    // If a format was passed on the command line, use that.
    // If not, use some default format based on parameters precision.
    let format = match options.format {
        Some(str) => Format::<num_format::Float, &ExtendedBigDecimal>::parse(str)?,
        None => {
            let padding = if options.equal_width {
                let precision_value = precision.unwrap_or(0);
                first
                    .num_integral_digits
                    .max(increment.num_integral_digits)
                    .max(last.num_integral_digits)
                    + if precision_value > 0 {
                        precision_value + 1
                    } else {
                        0
                    }
            } else {
                0
            };

            let formatter = match precision {
                // format with precision: decimal floats and integers
                Some(precision) => num_format::Float {
                    variant: FloatVariant::Decimal,
                    width: padding,
                    alignment: num_format::NumberAlignment::RightZero,
                    precision,
                    ..Default::default()
                },
                // format without precision: hexadecimal floats
                None => num_format::Float {
                    variant: FloatVariant::Shortest,
                    ..Default::default()
                },
            };
            Format::from_formatter(formatter)
        }
    };

    let result = print_seq(
        (first.number, increment.number, last.number),
        &options.separator,
        &options.terminator,
        &format,
    );
    match result {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == ErrorKind::BrokenPipe => Ok(()),
        Err(err) => Err(err.map_err_context(|| "write error".into())),
    }
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .trailing_var_arg(true)
        .infer_long_args(true)
        .version(uucore::crate_version!())
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
            // we use allow_hyphen_values instead of allow_negative_numbers because clap removed
            // the support for "exotic" negative numbers like -.1 (see https://github.com/clap-rs/clap/discussions/5837)
            Arg::new(ARG_NUMBERS)
                .allow_hyphen_values(true)
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

/// Floating point based code path
fn print_seq(
    range: RangeFloat,
    separator: &str,
    terminator: &str,
    format: &Format<num_format::Float, &ExtendedBigDecimal>,
) -> std::io::Result<()> {
    let stdout = stdout().lock();
    let mut stdout = BufWriter::new(stdout);
    let (first, increment, last) = range;
    let mut value = first;

    let mut is_first_iteration = true;
    while !done_printing(&value, &increment, &last) {
        if !is_first_iteration {
            stdout.write_all(separator.as_bytes())?;
        }
        format.fmt(&mut stdout, &value)?;
        // TODO Implement augmenting addition.
        value = value + increment.clone();
        is_first_iteration = false;
    }
    if !is_first_iteration {
        stdout.write_all(terminator.as_bytes())?;
    }
    stdout.flush()?;
    Ok(())
}
