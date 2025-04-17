// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (vars) NANOS numstr infinityh INFD nans nanh bigdecimal extendedbigdecimal
//! Parsing a duration from a string.
//!
//! Use the [`from_str`] function to parse a [`Duration`] from a string.

use crate::{
    display::Quotable,
    extendedbigdecimal::ExtendedBigDecimal,
    parser::num_parser::{self, ExtendedParserError, ParseTarget},
};
use num_traits::Signed;
use num_traits::ToPrimitive;
use num_traits::Zero;
use std::time::Duration;

/// Parse a duration from a string.
///
/// The string may contain only a number, like "123" or "4.5", or it
/// may contain a number with a unit specifier, like "123s" meaning
/// one hundred twenty three seconds or "4.5d" meaning four and a half
/// days. If no unit is specified, the unit is assumed to be seconds.
///
/// The only allowed suffixes are
///
/// * "s" for seconds,
/// * "m" for minutes,
/// * "h" for hours,
/// * "d" for days.
///
/// This function does not overflow if large values are provided. If
/// overflow would have occurred, [`Duration::MAX`] is returned instead.
///
/// If the value is smaller than 1 nanosecond, we return 1 nanosecond.
///
/// # Errors
///
/// This function returns an error if the input string is empty, the
/// input is not a valid number, or the unit specifier is invalid or
/// unknown.
///
/// # Examples
///
/// ```rust
/// use std::time::Duration;
/// use uucore::parser::parse_time::from_str;
/// assert_eq!(from_str("123"), Ok(Duration::from_secs(123)));
/// assert_eq!(from_str("2d"), Ok(Duration::from_secs(60 * 60 * 24 * 2)));
/// ```
pub fn from_str(string: &str) -> Result<Duration, String> {
    // TODO: Switch to Duration::NANOSECOND if that ever becomes stable
    // https://github.com/rust-lang/rust/issues/57391
    const NANOSECOND_DURATION: Duration = Duration::from_nanos(1);

    let len = string.len();
    if len == 0 {
        return Err(format!("invalid time interval {}", string.quote()));
    }
    let num = match num_parser::parse(
        string,
        ParseTarget::Duration,
        &[('s', 1), ('m', 60), ('h', 60 * 60), ('d', 60 * 60 * 24)],
    ) {
        Ok(ebd) | Err(ExtendedParserError::Overflow(ebd)) => ebd,
        Err(ExtendedParserError::Underflow(_)) => return Ok(NANOSECOND_DURATION),
        _ => {
            return Err(format!("invalid time interval {}", string.quote()));
        }
    };

    // Allow non-negative durations (-0 is fine), and infinity.
    let num = match num {
        ExtendedBigDecimal::BigDecimal(bd) if !bd.is_negative() => bd,
        ExtendedBigDecimal::MinusZero => 0.into(),
        ExtendedBigDecimal::Infinity => return Ok(Duration::MAX),
        _ => return Err(format!("invalid time interval {}", string.quote())),
    };

    // Transform to nanoseconds (9 digits after decimal point)
    let (nanos_bi, _) = num.with_scale(9).into_bigint_and_scale();

    // If the value is smaller than a nanosecond, just return that.
    if nanos_bi.is_zero() && !num.is_zero() {
        return Ok(NANOSECOND_DURATION);
    }

    const NANOS_PER_SEC: u32 = 1_000_000_000;
    let whole_secs: u64 = match (&nanos_bi / NANOS_PER_SEC).try_into() {
        Ok(whole_secs) => whole_secs,
        Err(_) => return Ok(Duration::MAX),
    };
    let nanos: u32 = (&nanos_bi % NANOS_PER_SEC).to_u32().unwrap();
    Ok(Duration::new(whole_secs, nanos))
}

#[cfg(test)]
mod tests {

    use crate::parser::parse_time::from_str;
    use std::time::Duration;

    #[test]
    fn test_no_units() {
        assert_eq!(from_str("123"), Ok(Duration::from_secs(123)));
    }

    #[test]
    fn test_units() {
        assert_eq!(from_str("2d"), Ok(Duration::from_secs(60 * 60 * 24 * 2)));
    }

    #[test]
    fn test_overflow() {
        // u64 seconds overflow (in Duration)
        assert_eq!(from_str("9223372036854775808d"), Ok(Duration::MAX));
        // ExtendedBigDecimal overflow
        assert_eq!(from_str("1e92233720368547758080"), Ok(Duration::MAX));
    }

    #[test]
    fn test_underflow() {
        // TODO: Switch to Duration::NANOSECOND if that ever becomes stable
        // https://github.com/rust-lang/rust/issues/57391
        const NANOSECOND_DURATION: Duration = Duration::from_nanos(1);

        // ExtendedBigDecimal underflow
        assert_eq!(from_str("1e-92233720368547758080"), Ok(NANOSECOND_DURATION));
        // nanoseconds underflow (in Duration)
        assert_eq!(from_str("0.0000000001"), Ok(NANOSECOND_DURATION));
        assert_eq!(from_str("1e-10"), Ok(NANOSECOND_DURATION));
        assert_eq!(from_str("9e-10"), Ok(NANOSECOND_DURATION));
        assert_eq!(from_str("1e-9"), Ok(NANOSECOND_DURATION));
        assert_eq!(from_str("1.9e-9"), Ok(NANOSECOND_DURATION));
        assert_eq!(from_str("2e-9"), Ok(Duration::from_nanos(2)));
    }

    #[test]
    fn test_zero() {
        assert_eq!(from_str("0e-9"), Ok(Duration::ZERO));
        assert_eq!(from_str("0e-100"), Ok(Duration::ZERO));
        assert_eq!(from_str("0e-92233720368547758080"), Ok(Duration::ZERO));
        assert_eq!(from_str("0.000000000000000000000"), Ok(Duration::ZERO));
    }

    #[test]
    fn test_hex_float() {
        assert_eq!(
            from_str("0x1.1p-1"),
            Ok(Duration::from_secs_f64(0.53125f64))
        );
        assert_eq!(
            from_str("0x1.1p-1d"),
            Ok(Duration::from_secs_f64(0.53125f64 * 3600.0 * 24.0))
        );
        assert_eq!(from_str("0xfh"), Ok(Duration::from_secs(15 * 3600)));
    }

    #[test]
    fn test_error_empty() {
        assert!(from_str("").is_err());
    }

    #[test]
    fn test_error_invalid_unit() {
        assert!(from_str("123X").is_err());
    }

    #[test]
    fn test_error_multi_bytes_characters() {
        assert!(from_str("10€").is_err());
    }

    #[test]
    fn test_error_invalid_magnitude() {
        assert!(from_str("12abc3s").is_err());
    }

    #[test]
    fn test_negative() {
        assert!(from_str("-1").is_err());
    }

    #[test]
    fn test_infinity() {
        assert_eq!(from_str("inf"), Ok(Duration::MAX));
        assert_eq!(from_str("infinity"), Ok(Duration::MAX));
        assert_eq!(from_str("infinityh"), Ok(Duration::MAX));
        assert_eq!(from_str("INF"), Ok(Duration::MAX));
        assert_eq!(from_str("INFs"), Ok(Duration::MAX));
    }

    #[test]
    fn test_nan() {
        assert!(from_str("nan").is_err());
        assert!(from_str("nans").is_err());
        assert!(from_str("-nanh").is_err());
        assert!(from_str("NAN").is_err());
        assert!(from_str("-NAN").is_err());
    }

    /// Test that capital letters are not allowed in suffixes.
    #[test]
    fn test_no_capital_letters() {
        assert!(from_str("1S").is_err());
        assert!(from_str("1M").is_err());
        assert!(from_str("1H").is_err());
        assert!(from_str("1D").is_err());
        assert!(from_str("INFD").is_err());
    }
}
