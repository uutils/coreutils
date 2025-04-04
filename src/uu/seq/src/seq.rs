// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore (ToDO) bigdecimal extendedbigdecimal numberparse hexadecimalfloat biguint
use std::ffi::OsString;
use std::io::{BufWriter, ErrorKind, Write, stdout};

use clap::{Arg, ArgAction, Command};
use num_bigint::BigUint;
use num_traits::ToPrimitive;
use num_traits::Zero;

use uucore::error::{FromIo, UResult};
use uucore::extendedbigdecimal::ExtendedBigDecimal;
use uucore::format::num_format::FloatVariant;
use uucore::format::{Format, num_format};
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

    if options.equal_width && options.format.is_some() {
        return Err(SeqError::FormatAndEqualWidth.into());
    }

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

    // If a format was passed on the command line, use that.
    // If not, use some default format based on parameters precision.
    let (format, padding, fast_allowed) = match options.format {
        Some(str) => (
            Format::<num_format::Float, &ExtendedBigDecimal>::parse(str)?,
            0,
            false,
        ),
        None => {
            let precision = select_precision(&first, &increment, &last);

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
                    precision: Some(precision),
                    ..Default::default()
                },
                // format without precision: hexadecimal floats
                None => num_format::Float {
                    variant: FloatVariant::Shortest,
                    ..Default::default()
                },
            };
            // Allow fast printing if precision is 0 (integer inputs), `print_seq` will do further checks.
            (
                Format::from_formatter(formatter),
                padding,
                precision == Some(0),
            )
        }
    };

    let result = print_seq(
        (first.number, increment.number, last.number),
        &options.separator,
        &options.terminator,
        &format,
        fast_allowed,
        padding,
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

/// Fast code path increment function.
///
/// Add inc to the string val[start..end]. This operates on ASCII digits, assuming
/// val and inc are well formed.
///
/// Returns the new value for start (can be less that the original value if we
/// have a carry or if inc > start).
///
/// We also assume that there is enough space in val to expand if start needs
/// to be updated.
fn fast_inc(val: &mut [u8], start: usize, end: usize, inc: &[u8]) -> usize {
    // To avoid a lot of casts to signed integers, we make sure to decrement pos
    // as late as possible, so that it does not ever go negative.
    let mut pos = end;
    let mut carry = 0u8;

    // First loop, add all digits of inc into val.
    for inc_pos in (0..inc.len()).rev() {
        pos -= 1;

        let mut new_val = inc[inc_pos] + carry;
        // Be careful here, only add existing digit of val.
        if pos >= start {
            new_val += val[pos] - b'0';
        }
        if new_val > b'9' {
            carry = 1;
            new_val -= 10;
        } else {
            carry = 0;
        }
        val[pos] = new_val;
    }

    // Done, now, if we have a carry, add that to the upper digits of val.
    if carry == 0 {
        return start.min(pos);
    }
    while pos > start {
        pos -= 1;

        if val[pos] == b'9' {
            // 9+1 = 10. Carry propagating, keep going.
            val[pos] = b'0';
        } else {
            // Carry stopped propagating, return unchanged start.
            val[pos] += 1;
            return start;
        }
    }

    // The carry propagated so far that a new digit was added.
    val[start - 1] = b'1';
    start - 1
}

/// Integer print, default format, positive increment: fast code path
/// that avoids reformating digit at all iterations.
fn fast_print_seq(
    mut stdout: impl Write,
    first: &BigUint,
    increment: u64,
    last: &BigUint,
    separator: &str,
    terminator: &str,
    padding: usize,
) -> std::io::Result<()> {
    // Nothing to do, just return.
    if last < first {
        return Ok(());
    }

    // Do at most u64::MAX loops. We can print in the order of 1e8 digits per second,
    // u64::MAX is 1e19, so it'd take hundreds of years for this to complete anyway.
    // TODO: we can move this test to `print_seq` if we care about this case.
    let loop_cnt = ((last - first) / increment).to_u64().unwrap_or(u64::MAX);

    // Format the first number.
    let first_str = first.to_string();

    // Makeshift log10.ceil
    let last_length = last.to_string().len();

    // Allocate a large u8 buffer, that contains a preformatted string
    // of the number followed by the `separator`.
    //
    // | ... head space ... | number | separator |
    // ^0                   ^ start  ^ num_end   ^ size (==buf.len())
    //
    // We keep track of start in this buffer, as the number grows.
    // When printing, we take a slice between start and end.
    let size = last_length.max(padding) + separator.len();
    // Fill with '0', this is needed for equal_width, and harmless otherwise.
    let mut buf = vec![b'0'; size];
    let buf = buf.as_mut_slice();

    let num_end = buf.len() - separator.len();
    let mut start = num_end - first_str.len();

    // Initialize buf with first and separator.
    buf[start..num_end].copy_from_slice(first_str.as_bytes());
    buf[num_end..].copy_from_slice(separator.as_bytes());

    // Normally, if padding is > 0, it should be equal to last_length,
    // so start would be == 0, but there are corner cases.
    start = start.min(num_end - padding);

    // Prepare the number to increment with as a string
    let inc_str = increment.to_string();
    let inc_str = inc_str.as_bytes();

    for _ in 0..loop_cnt {
        stdout.write_all(&buf[start..])?;
        start = fast_inc(buf, start, num_end, inc_str);
    }
    // Write the last number without separator, but with terminator.
    stdout.write_all(&buf[start..num_end])?;
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

/// Arbitrary precision decimal number code path ("slow" path)
fn print_seq(
    range: RangeFloat,
    separator: &str,
    terminator: &str,
    format: &Format<num_format::Float, &ExtendedBigDecimal>,
    fast_allowed: bool,
    padding: usize, // Used by fast path only
) -> std::io::Result<()> {
    let stdout = stdout().lock();
    let mut stdout = BufWriter::new(stdout);
    let (first, increment, last) = range;

    if fast_allowed {
        // Test if we can use fast code path.
        // First try to convert the range to BigUint (u64 for the increment).
        let (first_bui, increment_u64, last_bui) = (
            first.to_biguint(),
            increment.to_biguint().and_then(|x| x.to_u64()),
            last.to_biguint(),
        );
        if let (Some(first_bui), Some(increment_u64), Some(last_bui)) =
            (first_bui, increment_u64, last_bui)
        {
            return fast_print_seq(
                stdout,
                &first_bui,
                increment_u64,
                &last_bui,
                separator,
                terminator,
                padding,
            );
        }
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

#[cfg(test)]
mod tests {
    #[test]
    fn test_fast_inc_simple() {
        use crate::fast_inc;

        let mut val = [b'.', b'.', b'.', b'0', b'_'];
        let inc = [b'4'].as_ref();
        assert_eq!(fast_inc(val.as_mut(), 3, 4, inc), 3);
        assert_eq!(val, "...4_".as_bytes());
        assert_eq!(fast_inc(val.as_mut(), 3, 4, inc), 3);
        assert_eq!(val, "...8_".as_bytes());
        assert_eq!(fast_inc(val.as_mut(), 3, 4, inc), 2); // carried 1 more digit
        assert_eq!(val, "..12_".as_bytes());

        let mut val = [b'0', b'_'];
        let inc = [b'2'].as_ref();
        assert_eq!(fast_inc(val.as_mut(), 0, 1, inc), 0);
        assert_eq!(val, "2_".as_bytes());
        assert_eq!(fast_inc(val.as_mut(), 0, 1, inc), 0);
        assert_eq!(val, "4_".as_bytes());
        assert_eq!(fast_inc(val.as_mut(), 0, 1, inc), 0);
        assert_eq!(val, "6_".as_bytes());
    }

    // Check that we handle increment > val correctly.
    #[test]
    fn test_fast_inc_large_inc() {
        use crate::fast_inc;

        let mut val = [b'.', b'.', b'.', b'7', b'_'];
        let inc = "543".as_bytes();
        assert_eq!(fast_inc(val.as_mut(), 3, 4, inc), 1); // carried 2 more digits
        assert_eq!(val, ".550_".as_bytes());
        assert_eq!(fast_inc(val.as_mut(), 1, 4, inc), 0); // carried 1 more digit
        assert_eq!(val, "1093_".as_bytes());
    }

    // Check that we handle longer carries
    #[test]
    fn test_fast_inc_carry() {
        use crate::fast_inc;

        let mut val = [b'.', b'9', b'9', b'9', b'_'];
        let inc = "1".as_bytes();
        assert_eq!(fast_inc(val.as_mut(), 1, 4, inc), 0);
        assert_eq!(val, "1000_".as_bytes());

        let mut val = [b'.', b'9', b'9', b'9', b'_'];
        let inc = "11".as_bytes();
        assert_eq!(fast_inc(val.as_mut(), 1, 4, inc), 0);
        assert_eq!(val, "1010_".as_bytes());
    }
}
