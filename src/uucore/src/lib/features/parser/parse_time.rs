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
/// If `allow_suffixes` is true, the allowed suffixes are
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
/// assert_eq!(from_str("123", true), Ok(Duration::from_secs(123)));
/// assert_eq!(from_str("123", false), Ok(Duration::from_secs(123)));
/// assert_eq!(from_str("2d", true), Ok(Duration::from_secs(60 * 60 * 24 * 2)));
/// assert!(from_str("2d", false).is_err());
/// ```
pub fn from_str(string: &str, allow_suffixes: bool) -> Result<Duration, String> {
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
        if allow_suffixes {
            &[('s', 1), ('m', 60), ('h', 60 * 60), ('d', 60 * 60 * 24)]
        } else {
            &[]
        },
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
        assert_eq!(from_str("123", true), Ok(Duration::from_secs(123)));
        assert_eq!(from_str("123", false), Ok(Duration::from_secs(123)));
    }

    #[test]
    fn test_units() {
        assert_eq!(
            from_str("2d", true),
            Ok(Duration::from_secs(60 * 60 * 24 * 2))
        );
        assert!(from_str("2d", false).is_err());
    }

    #[test]
    fn test_overflow() {
        // u64 seconds overflow (in Duration)
        assert_eq!(from_str("9223372036854775808d", true), Ok(Duration::MAX));
        // ExtendedBigDecimal overflow
        assert_eq!(from_str("1e92233720368547758080", false), Ok(Duration::MAX));
        assert_eq!(from_str("1e92233720368547758080", false), Ok(Duration::MAX));
    }

    #[test]
    fn test_underflow() {
        // TODO: Switch to Duration::NANOSECOND if that ever becomes stable
        // https://github.com/rust-lang/rust/issues/57391
        const NANOSECOND_DURATION: Duration = Duration::from_nanos(1);

        // ExtendedBigDecimal underflow
        assert_eq!(
            from_str("1e-92233720368547758080", true),
            Ok(NANOSECOND_DURATION)
        );
        // nanoseconds underflow (in Duration, true)
        assert_eq!(from_str("0.0000000001", true), Ok(NANOSECOND_DURATION));
        assert_eq!(from_str("1e-10", true), Ok(NANOSECOND_DURATION));
        assert_eq!(from_str("9e-10", true), Ok(NANOSECOND_DURATION));
        assert_eq!(from_str("1e-9", true), Ok(NANOSECOND_DURATION));
        assert_eq!(from_str("1.9e-9", true), Ok(NANOSECOND_DURATION));
        assert_eq!(from_str("2e-9", true), Ok(Duration::from_nanos(2)));

        // ExtendedBigDecimal underflow
        assert_eq!(
            from_str("1e-92233720368547758080", false),
            Ok(NANOSECOND_DURATION)
        );
        // nanoseconds underflow (in Duration, false)
        assert_eq!(from_str("0.0000000001", false), Ok(NANOSECOND_DURATION));
        assert_eq!(from_str("1e-10", false), Ok(NANOSECOND_DURATION));
        assert_eq!(from_str("9e-10", false), Ok(NANOSECOND_DURATION));
        assert_eq!(from_str("1e-9", false), Ok(NANOSECOND_DURATION));
        assert_eq!(from_str("1.9e-9", false), Ok(NANOSECOND_DURATION));
        assert_eq!(from_str("2e-9", false), Ok(Duration::from_nanos(2)));
    }

    #[test]
    fn test_zero() {
        assert_eq!(from_str("0e-9", true), Ok(Duration::ZERO));
        assert_eq!(from_str("0e-100", true), Ok(Duration::ZERO));
        assert_eq!(
            from_str("0e-92233720368547758080", true),
            Ok(Duration::ZERO)
        );
        assert_eq!(
            from_str("0.000000000000000000000", true),
            Ok(Duration::ZERO)
        );

        assert_eq!(from_str("0e-9", false), Ok(Duration::ZERO));
        assert_eq!(from_str("0e-100", false), Ok(Duration::ZERO));
        assert_eq!(
            from_str("0e-92233720368547758080", false),
            Ok(Duration::ZERO)
        );
        assert_eq!(
            from_str("0.000000000000000000000", false),
            Ok(Duration::ZERO)
        );
    }

    #[test]
    fn test_hex_float() {
        assert_eq!(
            from_str("0x1.1p-1", true),
            Ok(Duration::from_secs_f64(0.53125f64))
        );
        assert_eq!(
            from_str("0x1.1p-1", false),
            Ok(Duration::from_secs_f64(0.53125f64))
        );
        assert_eq!(
            from_str("0x1.1p-1d", true),
            Ok(Duration::from_secs_f64(0.53125f64 * 3600.0 * 24.0))
        );
        assert_eq!(from_str("0xfh", true), Ok(Duration::from_secs(15 * 3600)));
    }

    #[test]
    fn test_error_empty() {
        assert!(from_str("", true).is_err());
        assert!(from_str("", false).is_err());
    }

    #[test]
    fn test_error_invalid_unit() {
        assert!(from_str("123X", true).is_err());
        assert!(from_str("123X", false).is_err());
    }

    #[test]
    fn test_error_multi_bytes_characters() {
        assert!(from_str("10€", true).is_err());
        assert!(from_str("10€", false).is_err());
    }

    #[test]
    fn test_error_invalid_magnitude() {
        assert!(from_str("12abc3s", true).is_err());
        assert!(from_str("12abc3s", false).is_err());
    }

    #[test]
    fn test_error_only_point() {
        assert!(from_str(".", true).is_err());
        assert!(from_str(".", false).is_err());
    }

    #[test]
    fn test_negative() {
        assert!(from_str("-1", true).is_err());
        assert!(from_str("-1", false).is_err());
    }

    #[test]
    fn test_infinity() {
        assert_eq!(from_str("inf", true), Ok(Duration::MAX));
        assert_eq!(from_str("infinity", true), Ok(Duration::MAX));
        assert_eq!(from_str("infinityh", true), Ok(Duration::MAX));
        assert_eq!(from_str("INF", true), Ok(Duration::MAX));
        assert_eq!(from_str("INFs", true), Ok(Duration::MAX));

        assert_eq!(from_str("inf", false), Ok(Duration::MAX));
        assert_eq!(from_str("infinity", false), Ok(Duration::MAX));
        assert_eq!(from_str("INF", false), Ok(Duration::MAX));
    }

    #[test]
    fn test_nan() {
        assert!(from_str("nan", true).is_err());
        assert!(from_str("nans", true).is_err());
        assert!(from_str("-nanh", true).is_err());
        assert!(from_str("NAN", true).is_err());
        assert!(from_str("-NAN", true).is_err());

        assert!(from_str("nan", false).is_err());
        assert!(from_str("NAN", false).is_err());
        assert!(from_str("-NAN", false).is_err());
    }

    /// Test that capital letters are not allowed in suffixes.
    #[test]
    fn test_no_capital_letters() {
        assert!(from_str("1S", true).is_err());
        assert!(from_str("1M", true).is_err());
        assert!(from_str("1H", true).is_err());
        assert!(from_str("1D", true).is_err());
        assert!(from_str("INFD", true).is_err());
    }
}
