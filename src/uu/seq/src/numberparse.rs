// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore extendedbigdecimal bigdecimal numberparse hexadecimalfloat
//! Parsing numbers for use in `seq`.
//!
//! This module provides an implementation of [`FromStr`] for the
//! [`PreciseNumber`] struct.
use std::str::FromStr;

use uucore::parser::num_parser::{ExtendedParser, ExtendedParserError};

use crate::number::PreciseNumber;
use uucore::extendedbigdecimal::ExtendedBigDecimal;

/// An error returned when parsing a number fails.
#[derive(Debug, PartialEq, Eq)]
pub enum ParseNumberError {
    Float,
    Nan,
}

/// Compute the number of integral and fractional digits in input string,
/// and wrap the result in a PreciseNumber.
/// We know that the string has already been parsed correctly, so we don't
/// need to be too careful.
fn compute_num_digits(input: &str, ebd: ExtendedBigDecimal) -> PreciseNumber {
    let input = input.to_lowercase();
    let input = input.trim_start();

    // Leading + is ignored for this.
    let input = input.strip_prefix('+').unwrap_or(input);

    // Integral digits for any hex number is ill-defined (0 is fine as an output)
    // Fractional digits for an floating hex number is ill-defined, return None
    // as we'll totally ignore that number for precision computations.
    // Still return 0 for hex integers though.
    if input.starts_with("0x") || input.starts_with("-0x") {
        return PreciseNumber {
            number: ebd,
            num_integral_digits: 0,
            num_fractional_digits: if input.contains('.') || input.contains('p') {
                None
            } else {
                Some(0)
            },
        };
    }

    // Split the exponent part, if any
    let parts: Vec<&str> = input.split('e').collect();
    debug_assert!(parts.len() <= 2);

    // Count all the digits up to `.`, `-` sign is included.
    let (mut int_digits, mut frac_digits) = match parts[0].find('.') {
        Some(i) => {
            // Cover special case .X and -.X where we behave as if there was a leading 0:
            // 0.X, -0.X.
            let int_digits = match i {
                0 => 1,
                1 if parts[0].starts_with('-') => 2,
                _ => i,
            };

            (int_digits, parts[0].len() - i - 1)
        }
        None => (parts[0].len(), 0),
    };

    // If there is an exponent, reparse that (yes this is not optimal,
    // but we can't necessarily exactly recover that from the parsed number).
    if parts.len() == 2 {
        let exp = parts[1].parse::<i64>().unwrap_or(0);
        // For positive exponents, effectively expand the number. Ignore negative exponents.
        // Also ignore overflowed exponents (unwrap_or(0)).
        if exp > 0 {
            int_digits += exp.try_into().unwrap_or(0);
        }
        frac_digits = if exp < frac_digits as i64 {
            // Subtract from i128 to avoid any overflow
            (frac_digits as i128 - exp as i128).try_into().unwrap_or(0)
        } else {
            0
        }
    }

    PreciseNumber {
        number: ebd,
        num_integral_digits: int_digits,
        num_fractional_digits: Some(frac_digits),
    }
}

// Note: We could also have provided an `ExtendedParser` implementation for
// PreciseNumber, but we want a simpler custom error.
impl FromStr for PreciseNumber {
    type Err = ParseNumberError;
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let ebd = match ExtendedBigDecimal::extended_parse(input) {
            Ok(ebd) => match ebd {
                // Handle special values
                ExtendedBigDecimal::BigDecimal(_) | ExtendedBigDecimal::MinusZero => {
                    // TODO: GNU `seq` treats small numbers < 1e-4950 as 0, we could do the same
                    // to avoid printing senselessly small numbers.
                    ebd
                }
                ExtendedBigDecimal::Infinity | ExtendedBigDecimal::MinusInfinity => {
                    return Ok(Self {
                        number: ebd,
                        num_integral_digits: 0,
                        num_fractional_digits: Some(0),
                    });
                }
                ExtendedBigDecimal::Nan | ExtendedBigDecimal::MinusNan => {
                    return Err(ParseNumberError::Nan);
                }
            },
            Err(ExtendedParserError::Underflow(ebd)) => ebd, // Treat underflow as 0
            Err(_) => return Err(ParseNumberError::Float),
        };

        Ok(compute_num_digits(input, ebd))
    }
}

#[cfg(test)]
mod tests {
    use bigdecimal::BigDecimal;
    use uucore::extendedbigdecimal::ExtendedBigDecimal;

    use crate::number::PreciseNumber;
    use crate::numberparse::ParseNumberError;

    /// Convenience function for parsing a [`Number`] and unwrapping.
    fn parse(s: &str) -> ExtendedBigDecimal {
        s.parse::<PreciseNumber>().unwrap().number
    }

    /// Convenience function for getting the number of integral digits.
    fn num_integral_digits(s: &str) -> usize {
        s.parse::<PreciseNumber>().unwrap().num_integral_digits
    }

    /// Convenience function for getting the number of fractional digits.
    fn num_fractional_digits(s: &str) -> usize {
        s.parse::<PreciseNumber>()
            .unwrap()
            .num_fractional_digits
            .unwrap()
    }

    /// Convenience function for making sure the number of fractional digits is "None"
    fn num_fractional_digits_is_none(s: &str) -> bool {
        s.parse::<PreciseNumber>()
            .unwrap()
            .num_fractional_digits
            .is_none()
    }

    #[test]
    fn test_parse_minus_zero_int() {
        assert_eq!(parse("-0e0"), ExtendedBigDecimal::MinusZero);
        assert_eq!(parse("-0e-0"), ExtendedBigDecimal::MinusZero);
        assert_eq!(parse("-0e1"), ExtendedBigDecimal::MinusZero);
        assert_eq!(parse("-0e+1"), ExtendedBigDecimal::MinusZero);
        assert_eq!(parse("-0.0e1"), ExtendedBigDecimal::MinusZero);
        assert_eq!(parse("-0x0"), ExtendedBigDecimal::MinusZero);
    }

    #[test]
    fn test_parse_minus_zero_float() {
        assert_eq!(parse("-0.0"), ExtendedBigDecimal::MinusZero);
        assert_eq!(parse("-0e-1"), ExtendedBigDecimal::MinusZero);
        assert_eq!(parse("-0.0e-1"), ExtendedBigDecimal::MinusZero);
    }

    #[test]
    fn test_parse_big_int() {
        assert_eq!(parse("0"), ExtendedBigDecimal::zero());
        assert_eq!(parse("0.1e1"), ExtendedBigDecimal::one());
        assert_eq!(parse("0.1E1"), ExtendedBigDecimal::one());
        assert_eq!(
            parse("1.0e1"),
            ExtendedBigDecimal::BigDecimal("10".parse::<BigDecimal>().unwrap())
        );
    }

    #[test]
    fn test_parse_hexadecimal_big_int() {
        assert_eq!(parse("0x0"), ExtendedBigDecimal::zero());
        assert_eq!(
            parse("0x10"),
            ExtendedBigDecimal::BigDecimal("16".parse::<BigDecimal>().unwrap())
        );
    }

    #[test]
    fn test_parse_big_decimal() {
        assert_eq!(
            parse("0.0"),
            ExtendedBigDecimal::BigDecimal("0.0".parse::<BigDecimal>().unwrap())
        );
        assert_eq!(
            parse(".0"),
            ExtendedBigDecimal::BigDecimal("0.0".parse::<BigDecimal>().unwrap())
        );
        assert_eq!(
            parse("1.0"),
            ExtendedBigDecimal::BigDecimal("1.0".parse::<BigDecimal>().unwrap())
        );
        assert_eq!(
            parse("10e-1"),
            ExtendedBigDecimal::BigDecimal("1.0".parse::<BigDecimal>().unwrap())
        );
        assert_eq!(
            parse("-1e-3"),
            ExtendedBigDecimal::BigDecimal("-0.001".parse::<BigDecimal>().unwrap())
        );
    }

    #[test]
    fn test_parse_inf() {
        assert_eq!(parse("inf"), ExtendedBigDecimal::Infinity);
        assert_eq!(parse("infinity"), ExtendedBigDecimal::Infinity);
        assert_eq!(parse("+inf"), ExtendedBigDecimal::Infinity);
        assert_eq!(parse("+infinity"), ExtendedBigDecimal::Infinity);
        assert_eq!(parse("-inf"), ExtendedBigDecimal::MinusInfinity);
        assert_eq!(parse("-infinity"), ExtendedBigDecimal::MinusInfinity);
    }

    #[test]
    fn test_parse_invalid_float() {
        assert_eq!(
            "1.2.3".parse::<PreciseNumber>().unwrap_err(),
            ParseNumberError::Float
        );
        assert_eq!(
            "1e2e3".parse::<PreciseNumber>().unwrap_err(),
            ParseNumberError::Float
        );
        assert_eq!(
            "1e2.3".parse::<PreciseNumber>().unwrap_err(),
            ParseNumberError::Float
        );
        assert_eq!(
            "-+-1".parse::<PreciseNumber>().unwrap_err(),
            ParseNumberError::Float
        );
    }

    #[test]
    fn test_parse_invalid_hex() {
        assert_eq!(
            "0xg".parse::<PreciseNumber>().unwrap_err(),
            ParseNumberError::Float
        );
    }

    #[test]
    fn test_parse_invalid_nan() {
        assert_eq!(
            "nan".parse::<PreciseNumber>().unwrap_err(),
            ParseNumberError::Nan
        );
        assert_eq!(
            "NAN".parse::<PreciseNumber>().unwrap_err(),
            ParseNumberError::Nan
        );
        assert_eq!(
            "NaN".parse::<PreciseNumber>().unwrap_err(),
            ParseNumberError::Nan
        );
        assert_eq!(
            "nAn".parse::<PreciseNumber>().unwrap_err(),
            ParseNumberError::Nan
        );
        assert_eq!(
            "-nan".parse::<PreciseNumber>().unwrap_err(),
            ParseNumberError::Nan
        );
    }

    #[test]
    #[allow(clippy::cognitive_complexity)]
    fn test_num_integral_digits() {
        // no decimal, no exponent
        assert_eq!(num_integral_digits("123"), 3);
        // decimal, no exponent
        assert_eq!(num_integral_digits("123.45"), 3);
        assert_eq!(num_integral_digits("-0.1"), 2);
        assert_eq!(num_integral_digits("-.1"), 2);
        // exponent, no decimal
        assert_eq!(num_integral_digits("123e4"), 3 + 4);
        assert_eq!(num_integral_digits("123e-4"), 3);
        assert_eq!(num_integral_digits("-1e-3"), 2);
        // decimal and exponent
        assert_eq!(num_integral_digits("123.45e6"), 3 + 6);
        assert_eq!(num_integral_digits("123.45e-6"), 3);
        assert_eq!(num_integral_digits("123.45e-1"), 3);
        assert_eq!(num_integral_digits("-0.1e0"), 2);
        assert_eq!(num_integral_digits("-0.1e2"), 4);
        assert_eq!(num_integral_digits("-.1e0"), 2);
        assert_eq!(num_integral_digits("-.1e2"), 4);
        assert_eq!(num_integral_digits("-1.e-3"), 2);
        assert_eq!(num_integral_digits("-1.0e-4"), 2);
        // minus zero int
        assert_eq!(num_integral_digits("-0e0"), 2);
        assert_eq!(num_integral_digits("-0e-0"), 2);
        assert_eq!(num_integral_digits("-0e1"), 3);
        assert_eq!(num_integral_digits("-0e+1"), 3);
        assert_eq!(num_integral_digits("-0.0e1"), 3);
        // minus zero float
        assert_eq!(num_integral_digits("-0.0"), 2);
        assert_eq!(num_integral_digits("-0e-1"), 2);
        assert_eq!(num_integral_digits("-0.0e-1"), 2);

        // TODO In GNU `seq`, the `-w` option does not seem to work with
        // hexadecimal arguments. In order to match that behavior, we
        // report the number of integral digits as zero for hexadecimal
        // inputs.
        assert_eq!(num_integral_digits("0xff"), 0);
    }

    #[test]
    #[allow(clippy::cognitive_complexity)]
    fn test_num_fractional_digits() {
        // no decimal, no exponent
        assert_eq!(num_fractional_digits("123"), 0);
        assert_eq!(num_fractional_digits("0xff"), 0);
        // decimal, no exponent
        assert_eq!(num_fractional_digits("123.45"), 2);
        assert_eq!(num_fractional_digits("-0.1"), 1);
        assert_eq!(num_fractional_digits("-.1"), 1);
        // exponent, no decimal
        assert_eq!(num_fractional_digits("123e4"), 0);
        assert_eq!(num_fractional_digits("123e-4"), 4);
        assert_eq!(num_fractional_digits("123e-1"), 1);
        assert_eq!(num_fractional_digits("-1e-3"), 3);
        // decimal and exponent
        assert_eq!(num_fractional_digits("123.45e6"), 0);
        assert_eq!(num_fractional_digits("123.45e1"), 1);
        assert_eq!(num_fractional_digits("123.45e-6"), 8);
        assert_eq!(num_fractional_digits("123.45e-1"), 3);
        assert_eq!(num_fractional_digits("-0.1e0"), 1);
        assert_eq!(num_fractional_digits("-0.1e2"), 0);
        assert_eq!(num_fractional_digits("-.1e0"), 1);
        assert_eq!(num_fractional_digits("-.1e2"), 0);
        assert_eq!(num_fractional_digits("-1.e-3"), 3);
        assert_eq!(num_fractional_digits("-1.0e-4"), 5);
        // minus zero int
        assert_eq!(num_fractional_digits("-0e0"), 0);
        assert_eq!(num_fractional_digits("-0e-0"), 0);
        assert_eq!(num_fractional_digits("-0e1"), 0);
        assert_eq!(num_fractional_digits("-0e+1"), 0);
        assert_eq!(num_fractional_digits("-0.0e1"), 0);
        // minus zero float
        assert_eq!(num_fractional_digits("-0.0"), 1);
        assert_eq!(num_fractional_digits("-0e-1"), 1);
        assert_eq!(num_fractional_digits("-0.0e-1"), 2);
        // Hexadecimal numbers
        assert_eq!(num_fractional_digits("0xff"), 0);
        assert!(num_fractional_digits_is_none("0xff.1"));
    }

    #[test]
    fn test_parse_min_exponents() {
        // Make sure exponents < i64::MIN do not cause errors
        assert!("1e-9223372036854775807".parse::<PreciseNumber>().is_ok());
        assert!("1e-9223372036854775808".parse::<PreciseNumber>().is_ok());
        assert!("1e-92233720368547758080".parse::<PreciseNumber>().is_ok());
    }

    #[test]
    fn test_parse_max_exponents() {
        // Make sure exponents much bigger than i64::MAX cause errors
        assert!("1e9223372036854775807".parse::<PreciseNumber>().is_ok());
        assert!("1e92233720368547758070".parse::<PreciseNumber>().is_err());
    }
}
