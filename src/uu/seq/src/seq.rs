// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore (ToDO) bigdecimal extendedbigdecimal numberparse hexadecimalfloat biguint
use std::ffi::OsString;
use std::io::{BufWriter, ErrorKind, Write, stdout};

use clap::{Arg, ArgAction, Command};
use num_bigint::BigUint;
use num_traits::Signed;
use num_traits::ToPrimitive;
use num_traits::Zero;

use uucore::error::{FromIo, UResult};
use uucore::format::num_format::FloatVariant;
use uucore::format::{ExtendedBigDecimal, Format, num_format};
use uucore::{format_usage, help_about, help_usage};

mod error;
mod hexadecimalfloat;

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
    first: Option<usize>,
    increment: Option<usize>,
    last: Option<usize>,
) -> Option<usize> {
    match (first, increment, last) {
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

    let (first, first_precision) = if numbers.len() > 1 {
        match numbers[0].parse() {
            Ok(num) => (num, hexadecimalfloat::parse_precision(numbers[0])),
            Err(e) => return Err(SeqError::ParseError(numbers[0].to_string(), e).into()),
        }
    } else {
        (PreciseNumber::one(), Some(0))
    };
    let (increment, increment_precision) = if numbers.len() > 2 {
        match numbers[1].parse() {
            Ok(num) => (num, hexadecimalfloat::parse_precision(numbers[1])),
            Err(e) => return Err(SeqError::ParseError(numbers[1].to_string(), e).into()),
        }
    } else {
        (PreciseNumber::one(), Some(0))
    };
    if increment.is_zero() {
        return Err(SeqError::ZeroIncrement(numbers[1].to_string()).into());
    }
    let (last, last_precision): (PreciseNumber, Option<usize>) = {
        // We are guaranteed that `numbers.len()` is greater than zero
        // and at most three because of the argument specification in
        // `uu_app()`.
        let n: usize = numbers.len();
        match numbers[n - 1].parse() {
            Ok(num) => (num, hexadecimalfloat::parse_precision(numbers[n - 1])),
            Err(e) => return Err(SeqError::ParseError(numbers[n - 1].to_string(), e).into()),
        }
    };

    let precision = select_precision(first_precision, increment_precision, last_precision);

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

    // Allow fast printing, `print_seq` will do further checks.
    let fast_allowed = options.format.is_none() && !options.equal_width && precision == Some(0);

    let result = print_seq(
        (first.number, increment.number, last.number),
        &options.separator,
        &options.terminator,
        &format,
        fast_allowed,
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

fn ebd_to_biguint(ebd: &ExtendedBigDecimal) -> Option<BigUint> {
    match ebd {
        ExtendedBigDecimal::BigDecimal(big_decimal) => {
            let (bi, scale) = big_decimal.as_bigint_and_scale();
            if bi.is_negative() || scale > 0 || scale < -(u32::MAX as i64) {
                return None;
            }
            bi.to_biguint()
                .map(|bi| bi * BigUint::from(10u32).pow(-scale as u32))
        }
        _ => None,
    }
}

// Add inc to the string val[start..end]. This operates on ASCII digits, assuming
// val and inc are well formed.
// Returns the new value for start.
// TODO: Add unit tests for this?
fn inc(val: &mut [u8], start: usize, end: usize, inc: &[u8]) -> usize {
    let mut pos = end - 1;
    let mut inc_pos = inc.len() - 1;
    let mut carry = 0u8;

    // First loop, add all digits of inc into val.
    loop {
        let mut new_val = (*inc)[inc_pos] + carry;
        // Be careful here, only add existing digit of val.
        if pos >= start {
            new_val += (*val)[pos] - b'0';
        }
        if new_val > b'9' {
            carry = 1;
            new_val -= 10;
        } else {
            carry = 0;
        }
        (*val)[pos] = new_val;

        pos -= 1;
        if inc_pos == 0 {
            break;
        }
        inc_pos -= 1;
    }

    // Done, now, if we have a carry, add that to the upper digits of val.
    if carry == 0 {
        return start.min(pos + 1);
    }
    loop {
        if pos < start {
            // The carry propagated so far that a new digit was added.
            (*val)[pos] = b'1';
            return pos; // == start - 1
        }

        if (*val)[pos] == b'9' {
            // 9+1 = 10. Carry propagating, keep going.
            (*val)[pos] = b'0';
        } else {
            // Carry stopped propagating, return unchanged start.
            (*val)[pos] += 1;
            return start;
        }

        pos -= 1;
    }
}

/// Integer print, default format, positive increment: fast code path
/// that avoids reformating digit at all iterations.
/// TODO: We could easily support equal_width (we do quite a bit of work
/// _not_ supporting that and aligning the number to the left).
fn print_seq_fast(
    mut stdout: impl Write,
    first: &BigUint,
    increment: u64,
    last: &BigUint,
    separator: &str,
    terminator: &str,
) -> std::io::Result<()> {
    // Nothing to do, just return.
    if last < first {
        return Ok(());
    }

    // Do at most u64::MAX loops. We can print in the order of 1e8 digits per second,
    // u64::MAX is 1e19, so it'd take hundreds of years for this to complete anyway.
    // TODO: we can move this test to `print_seq` if we care about this case.
    let loop_cnt = ((last - first) / increment).to_u64().unwrap_or(u64::MAX);
    let mut i = 0u64;

    // Format and print the first digit.
    let first_str = first.to_string();
    stdout.write_all(first_str.as_bytes())?;

    // Makeshift log10.ceil
    let last_length = last.to_string().len();

    // Allocate a large u8 buffer, that contains a preformatted string
    // of the `separator` followed by the number.
    //
    // | ... head space ... | separator | number |
    // ^0                   ^ start     ^ pos    ^ end (==buf.len())
    //
    // We keep track of 2 indices in this buffer: start and pos.
    // When printing, we take a slice between start and end.
    let end = separator.len() + last_length;
    let mut buf = vec![0u8; end];
    let buf = buf.as_mut_slice();

    let mut pos = end - first_str.len();
    let mut start = pos - separator.len();

    // Initialize buf with separator and first.
    buf[start..pos].copy_from_slice(separator.as_bytes());
    buf[pos..end].copy_from_slice(first_str.as_bytes());

    // Prepare the number to increment with as a string
    let inc_str = increment.to_string();
    let inc_str = inc_str.as_bytes();

    while i < loop_cnt {
        let new_pos = inc(buf, pos, end, inc_str);
        if pos != new_pos {
            // Number overflowed, move the position to the right.
            pos = new_pos;
            // Move the separator.
            start = new_pos - separator.len();
            buf[start..pos].copy_from_slice(separator.as_bytes());
        }
        i += 1;
        stdout.write_all(&buf[start..end])?;
    }
    write!(stdout, "{terminator}")?;
    stdout.flush()?;
    Ok(())
}

fn done_printing<T: Zero + PartialOrd>(next: &T, increment: &T, last: &T) -> bool {
    if increment >= &T::zero() {
        next > last
    } else {
        next < last
    }
}

/// Floating point based code path ("slow" path)
fn print_seq(
    range: RangeFloat,
    separator: &str,
    terminator: &str,
    format: &Format<num_format::Float, &ExtendedBigDecimal>,
    fast_allowed: bool,
) -> std::io::Result<()> {
    let stdout = stdout().lock();
    let mut stdout = BufWriter::new(stdout);
    let (first, increment, last) = range;

    // Test if we can use fast printing
    let (first_bui, increment_bui, last_bui) = (
        ebd_to_biguint(&first),
        ebd_to_biguint(&increment),
        ebd_to_biguint(&last),
    );

    // TODO: We could easily support last == infinity
    // Clippy wants to use `if let Some(...) = ...` to avoid is_some/unwrap combination, but that's
    // not possible within an "if" test with multiple sub-expressions.
    #[allow(clippy::unnecessary_unwrap)]
    if fast_allowed
        && first_bui.is_some()
        && increment_bui.is_some() // This implies increment is > 0
        && last_bui.is_some()
    {
        return print_seq_fast(
            stdout,
            &first_bui.unwrap(),
            increment_bui.unwrap().to_u64().unwrap(),
            &last_bui.unwrap(),
            separator,
            terminator,
        );
    }

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
