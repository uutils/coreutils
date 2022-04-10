// spell-checker:ignore extendedbigdecimal extendedbigint bigdecimal numberparse
//! Parsing numbers for use in `seq`.
//!
//! This module provides an implementation of [`FromStr`] for the
//! [`PreciseNumber`] struct.
use std::str::FromStr;

use bigdecimal::BigDecimal;
use num_bigint::BigInt;
use num_bigint::Sign;
use num_traits::Num;
use num_traits::Zero;

use crate::extendedbigdecimal::ExtendedBigDecimal;
use crate::extendedbigint::ExtendedBigInt;
use crate::number::Number;
use crate::number::PreciseNumber;

/// An error returned when parsing a number fails.
#[derive(Debug, PartialEq)]
pub enum ParseNumberError {
    Float,
    Nan,
    Hex,
}

/// Decide whether a given string and its parsed `BigInt` is negative zero.
fn is_minus_zero_int(s: &str, n: &BigInt) -> bool {
    s.starts_with('-') && n == &BigInt::zero()
}

/// Decide whether a given string and its parsed `BigDecimal` is negative zero.
fn is_minus_zero_float(s: &str, x: &BigDecimal) -> bool {
    s.starts_with('-') && x == &BigDecimal::zero()
}

/// Parse a number with neither a decimal point nor an exponent.
///
/// # Errors
///
/// This function returns an error if the input string is a variant of
/// "NaN" or if no [`BigInt`] could be parsed from the string.
///
/// # Examples
///
/// ```rust,ignore
/// let actual = "0".parse::<Number>().unwrap().number;
/// let expected = Number::BigInt(BigInt::zero());
/// assert_eq!(actual, expected);
/// ```
fn parse_no_decimal_no_exponent(s: &str) -> Result<PreciseNumber, ParseNumberError> {
    match s.parse::<BigInt>() {
        Ok(n) => {
            // If `s` is '-0', then `parse()` returns `BigInt::zero()`,
            // but we need to return `Number::MinusZeroInt` instead.
            if is_minus_zero_int(s, &n) {
                Ok(PreciseNumber::new(
                    Number::Int(ExtendedBigInt::MinusZero),
                    s.len(),
                    0,
                ))
            } else {
                Ok(PreciseNumber::new(
                    Number::Int(ExtendedBigInt::BigInt(n)),
                    s.len(),
                    0,
                ))
            }
        }
        Err(_) => {
            // Possibly "NaN" or "inf".
            //
            // TODO In Rust v1.53.0, this change
            // https://github.com/rust-lang/rust/pull/78618 improves the
            // parsing of floats to include being able to parse "NaN"
            // and "inf". So when the minimum version of this crate is
            // increased to 1.53.0, we should just use the built-in
            // `f32` parsing instead.
            if s.eq_ignore_ascii_case("inf") {
                Ok(PreciseNumber::new(
                    Number::Float(ExtendedBigDecimal::Infinity),
                    0,
                    0,
                ))
            } else if s.eq_ignore_ascii_case("-inf") {
                Ok(PreciseNumber::new(
                    Number::Float(ExtendedBigDecimal::MinusInfinity),
                    0,
                    0,
                ))
            } else if s.eq_ignore_ascii_case("nan") || s.eq_ignore_ascii_case("-nan") {
                Err(ParseNumberError::Nan)
            } else {
                Err(ParseNumberError::Float)
            }
        }
    }
}

/// Parse a number with an exponent but no decimal point.
///
/// # Errors
///
/// This function returns an error if `s` is not a valid number.
///
/// # Examples
///
/// ```rust,ignore
/// let actual = "1e2".parse::<Number>().unwrap().number;
/// let expected = "100".parse::<BigInt>().unwrap();
/// assert_eq!(actual, expected);
/// ```
fn parse_exponent_no_decimal(s: &str, j: usize) -> Result<PreciseNumber, ParseNumberError> {
    let exponent: i64 = s[j + 1..].parse().map_err(|_| ParseNumberError::Float)?;
    // If the exponent is strictly less than zero, then the number
    // should be treated as a floating point number that will be
    // displayed in decimal notation. For example, "1e-2" will be
    // displayed as "0.01", but "1e2" will be displayed as "100",
    // without a decimal point.
    let x: BigDecimal = s.parse().map_err(|_| ParseNumberError::Float)?;
    let num_integral_digits = if is_minus_zero_float(s, &x) {
        2
    } else {
        let total = j as i64 + exponent;
        let result = if total < 1 {
            1
        } else {
            total.try_into().unwrap()
        };
        if x.sign() == Sign::Minus {
            result + 1
        } else {
            result
        }
    };
    let num_fractional_digits = if exponent < 0 { -exponent as usize } else { 0 };

    if exponent < 0 {
        if is_minus_zero_float(s, &x) {
            Ok(PreciseNumber::new(
                Number::Float(ExtendedBigDecimal::MinusZero),
                num_integral_digits,
                num_fractional_digits,
            ))
        } else {
            Ok(PreciseNumber::new(
                Number::Float(ExtendedBigDecimal::BigDecimal(x)),
                num_integral_digits,
                num_fractional_digits,
            ))
        }
    } else {
        let zeros = "0".repeat(exponent.try_into().unwrap());
        let expanded = [&s[0..j], &zeros].concat();
        parse_no_decimal_no_exponent(&expanded)
    }
}

/// Parse a number with a decimal point but no exponent.
///
/// # Errors
///
/// This function returns an error if `s` is not a valid number.
///
/// # Examples
///
/// ```rust,ignore
/// let actual = "1.2".parse::<Number>().unwrap().number;
/// let expected = "1.2".parse::<BigDecimal>().unwrap();
/// assert_eq!(actual, expected);
/// ```
fn parse_decimal_no_exponent(s: &str, i: usize) -> Result<PreciseNumber, ParseNumberError> {
    let x: BigDecimal = s.parse().map_err(|_| ParseNumberError::Float)?;

    // The number of integral digits is the number of chars until the period.
    //
    // This includes the negative sign if there is one. Also, it is
    // possible that a number is expressed as "-.123" instead of
    // "-0.123", but when we display the number we want it to include
    // the leading 0.
    let num_integral_digits = if s.starts_with("-.") { i + 1 } else { i };
    let num_fractional_digits = s.len() - (i + 1);
    if is_minus_zero_float(s, &x) {
        Ok(PreciseNumber::new(
            Number::Float(ExtendedBigDecimal::MinusZero),
            num_integral_digits,
            num_fractional_digits,
        ))
    } else {
        Ok(PreciseNumber::new(
            Number::Float(ExtendedBigDecimal::BigDecimal(x)),
            num_integral_digits,
            num_fractional_digits,
        ))
    }
}

/// Parse a number with both a decimal point and an exponent.
///
/// # Errors
///
/// This function returns an error if `s` is not a valid number.
///
/// # Examples
///
/// ```rust,ignore
/// let actual = "1.2e3".parse::<Number>().unwrap().number;
/// let expected = "1200".parse::<BigInt>().unwrap();
/// assert_eq!(actual, expected);
/// ```
fn parse_decimal_and_exponent(
    s: &str,
    i: usize,
    j: usize,
) -> Result<PreciseNumber, ParseNumberError> {
    // Because of the match guard, this subtraction will not underflow.
    let num_digits_between_decimal_point_and_e = (j - (i + 1)) as i64;
    let exponent: i64 = s[j + 1..].parse().map_err(|_| ParseNumberError::Float)?;
    let val: BigDecimal = s.parse().map_err(|_| ParseNumberError::Float)?;

    let num_integral_digits = {
        let minimum: usize = {
            let integral_part: f64 = s[..j].parse().map_err(|_| ParseNumberError::Float)?;
            if integral_part.is_sign_negative() {
                2
            } else {
                1
            }
        };
        // Special case: if the string is "-.1e2", we need to treat it
        // as if it were "-0.1e2".
        let total = if s.starts_with("-.") {
            i as i64 + exponent + 1
        } else {
            i as i64 + exponent
        };
        if total < minimum as i64 {
            minimum
        } else {
            total.try_into().unwrap()
        }
    };

    let num_fractional_digits = if num_digits_between_decimal_point_and_e < exponent {
        0
    } else {
        (num_digits_between_decimal_point_and_e - exponent)
            .try_into()
            .unwrap()
    };

    if num_digits_between_decimal_point_and_e <= exponent {
        if is_minus_zero_float(s, &val) {
            Ok(PreciseNumber::new(
                Number::Int(ExtendedBigInt::MinusZero),
                num_integral_digits,
                num_fractional_digits,
            ))
        } else {
            let zeros: String = "0".repeat(
                (exponent - num_digits_between_decimal_point_and_e)
                    .try_into()
                    .unwrap(),
            );
            let expanded = [&s[0..i], &s[i + 1..j], &zeros].concat();
            let n = expanded
                .parse::<BigInt>()
                .map_err(|_| ParseNumberError::Float)?;
            Ok(PreciseNumber::new(
                Number::Int(ExtendedBigInt::BigInt(n)),
                num_integral_digits,
                num_fractional_digits,
            ))
        }
    } else if is_minus_zero_float(s, &val) {
        Ok(PreciseNumber::new(
            Number::Float(ExtendedBigDecimal::MinusZero),
            num_integral_digits,
            num_fractional_digits,
        ))
    } else {
        Ok(PreciseNumber::new(
            Number::Float(ExtendedBigDecimal::BigDecimal(val)),
            num_integral_digits,
            num_fractional_digits,
        ))
    }
}

/// Parse a hexadecimal integer from a string.
///
/// # Errors
///
/// This function returns an error if no [`BigInt`] could be parsed from
/// the string.
///
/// # Examples
///
/// ```rust,ignore
/// let actual = "0x0".parse::<Number>().unwrap().number;
/// let expected = Number::BigInt(BigInt::zero());
/// assert_eq!(actual, expected);
/// ```
fn parse_hexadecimal(s: &str) -> Result<PreciseNumber, ParseNumberError> {
    let (is_neg, s) = if s.starts_with('-') {
        (true, &s[3..])
    } else {
        (false, &s[2..])
    };

    if s.starts_with('-') || s.starts_with('+') {
        // Even though this is more like an invalid hexadecimal number,
        // GNU reports this as an invalid floating point number, so we
        // use `ParseNumberError::Float` to match that behavior.
        return Err(ParseNumberError::Float);
    }

    let num = BigInt::from_str_radix(s, 16).map_err(|_| ParseNumberError::Hex)?;

    match (is_neg, num == BigInt::zero()) {
        (true, true) => Ok(PreciseNumber::new(
            Number::Int(ExtendedBigInt::MinusZero),
            2,
            0,
        )),
        (true, false) => Ok(PreciseNumber::new(
            Number::Int(ExtendedBigInt::BigInt(-num)),
            0,
            0,
        )),
        (false, _) => Ok(PreciseNumber::new(
            Number::Int(ExtendedBigInt::BigInt(num)),
            0,
            0,
        )),
    }
}

impl FromStr for PreciseNumber {
    type Err = ParseNumberError;
    fn from_str(mut s: &str) -> Result<Self, Self::Err> {
        // Trim leading whitespace.
        s = s.trim_start();

        // Trim a single leading "+" character.
        if s.starts_with('+') {
            s = &s[1..];
        }

        // Check if the string seems to be in hexadecimal format.
        //
        // May be 0x123 or -0x123, so the index `i` may be either 0 or 1.
        if let Some(i) = s.to_lowercase().find("0x") {
            if i <= 1 {
                return parse_hexadecimal(s);
            }
        }

        // Find the decimal point and the exponent symbol. Parse the
        // number differently depending on its form. This is important
        // because the form of the input dictates how the output will be
        // presented.
        match (s.find('.'), s.find('e')) {
            // For example, "123456" or "inf".
            (None, None) => parse_no_decimal_no_exponent(s),
            // For example, "123e456" or "1e-2".
            (None, Some(j)) => parse_exponent_no_decimal(s, j),
            // For example, "123.456".
            (Some(i), None) => parse_decimal_no_exponent(s, i),
            // For example, "123.456e789".
            (Some(i), Some(j)) if i < j => parse_decimal_and_exponent(s, i, j),
            // For example, "1e2.3" or "1.2.3".
            _ => Err(ParseNumberError::Float),
        }
    }
}

#[cfg(test)]
mod tests {

    use bigdecimal::BigDecimal;
    use num_bigint::BigInt;
    use num_traits::Zero;

    use crate::extendedbigdecimal::ExtendedBigDecimal;
    use crate::extendedbigint::ExtendedBigInt;
    use crate::number::Number;
    use crate::number::PreciseNumber;
    use crate::numberparse::ParseNumberError;

    /// Convenience function for parsing a [`Number`] and unwrapping.
    fn parse(s: &str) -> Number {
        s.parse::<PreciseNumber>().unwrap().number
    }

    /// Convenience function for getting the number of integral digits.
    fn num_integral_digits(s: &str) -> usize {
        s.parse::<PreciseNumber>().unwrap().num_integral_digits
    }

    /// Convenience function for getting the number of fractional digits.
    fn num_fractional_digits(s: &str) -> usize {
        s.parse::<PreciseNumber>().unwrap().num_fractional_digits
    }

    #[test]
    fn test_parse_minus_zero_int() {
        assert_eq!(parse("-0e0"), Number::Int(ExtendedBigInt::MinusZero));
        assert_eq!(parse("-0e-0"), Number::Int(ExtendedBigInt::MinusZero));
        assert_eq!(parse("-0e1"), Number::Int(ExtendedBigInt::MinusZero));
        assert_eq!(parse("-0e+1"), Number::Int(ExtendedBigInt::MinusZero));
        assert_eq!(parse("-0.0e1"), Number::Int(ExtendedBigInt::MinusZero));
        assert_eq!(parse("-0x0"), Number::Int(ExtendedBigInt::MinusZero));
    }

    #[test]
    fn test_parse_minus_zero_float() {
        assert_eq!(parse("-0.0"), Number::Float(ExtendedBigDecimal::MinusZero));
        assert_eq!(parse("-0e-1"), Number::Float(ExtendedBigDecimal::MinusZero));
        assert_eq!(
            parse("-0.0e-1"),
            Number::Float(ExtendedBigDecimal::MinusZero)
        );
    }

    #[test]
    fn test_parse_big_int() {
        assert_eq!(parse("0"), Number::Int(ExtendedBigInt::zero()));
        assert_eq!(parse("0.1e1"), Number::Int(ExtendedBigInt::one()));
        assert_eq!(
            parse("1.0e1"),
            Number::Int(ExtendedBigInt::BigInt("10".parse::<BigInt>().unwrap()))
        );
    }

    #[test]
    fn test_parse_hexadecimal_big_int() {
        assert_eq!(parse("0x0"), Number::Int(ExtendedBigInt::zero()));
        assert_eq!(
            parse("0x10"),
            Number::Int(ExtendedBigInt::BigInt("16".parse::<BigInt>().unwrap()))
        );
    }

    #[test]
    fn test_parse_big_decimal() {
        assert_eq!(
            parse("0.0"),
            Number::Float(ExtendedBigDecimal::BigDecimal(
                "0.0".parse::<BigDecimal>().unwrap()
            ))
        );
        assert_eq!(
            parse(".0"),
            Number::Float(ExtendedBigDecimal::BigDecimal(
                "0.0".parse::<BigDecimal>().unwrap()
            ))
        );
        assert_eq!(
            parse("1.0"),
            Number::Float(ExtendedBigDecimal::BigDecimal(
                "1.0".parse::<BigDecimal>().unwrap()
            ))
        );
        assert_eq!(
            parse("10e-1"),
            Number::Float(ExtendedBigDecimal::BigDecimal(
                "1.0".parse::<BigDecimal>().unwrap()
            ))
        );
        assert_eq!(
            parse("-1e-3"),
            Number::Float(ExtendedBigDecimal::BigDecimal(
                "-0.001".parse::<BigDecimal>().unwrap()
            ))
        );
    }

    #[test]
    fn test_parse_inf() {
        assert_eq!(parse("inf"), Number::Float(ExtendedBigDecimal::Infinity));
        assert_eq!(parse("+inf"), Number::Float(ExtendedBigDecimal::Infinity));
        assert_eq!(
            parse("-inf"),
            Number::Float(ExtendedBigDecimal::MinusInfinity)
        );
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
            ParseNumberError::Hex
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
    fn test_num_integral_digits() {
        // no decimal, no exponent
        assert_eq!(num_integral_digits("123"), 3);
        // decimal, no exponent
        assert_eq!(num_integral_digits("123.45"), 3);
        assert_eq!(num_integral_digits("-0.1"), 2);
        assert_eq!(num_integral_digits("-.1"), 2);
        // exponent, no decimal
        assert_eq!(num_integral_digits("123e4"), 3 + 4);
        assert_eq!(num_integral_digits("123e-4"), 1);
        assert_eq!(num_integral_digits("-1e-3"), 2);
        // decimal and exponent
        assert_eq!(num_integral_digits("123.45e6"), 3 + 6);
        assert_eq!(num_integral_digits("123.45e-6"), 1);
        assert_eq!(num_integral_digits("123.45e-1"), 2);
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
    }
}
