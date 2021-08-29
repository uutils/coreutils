// spell-checker:ignore bigdecimal
//! A type to represent the possible start, increment, and end values for seq.
//!
//! The [`Number`] enumeration represents the possible values for the
//! start, increment, and end values for `seq`. These may be integers,
//! floating point numbers, negative zero, etc. A [`Number`] can be
//! parsed from a string by calling [`parse`].
use std::str::FromStr;

use bigdecimal::BigDecimal;
use bigdecimal::ParseBigDecimalError;
use num_bigint::BigInt;
use num_bigint::ParseBigIntError;
use num_traits::Zero;

use uucore::display::Quotable;

/// An integral or floating point number.
pub enum Number {
    /// Negative zero, as if it were an integer.
    MinusZeroInt,

    /// An arbitrary precision integer.
    BigInt(BigInt),

    /// Floating point negative zero.
    MinusZeroFloat,

    /// An arbitrary precision float.
    BigDecimal(BigDecimal),
}

impl Number {
    /// Decide whether this number is zero (either positive or negative).
    pub fn is_zero(&self) -> bool {
        match self {
            Number::MinusZeroInt => true,
            Number::MinusZeroFloat => true,
            Number::BigInt(n) => n.is_zero(),
            Number::BigDecimal(n) => n.is_zero(),
        }
    }

    /// Convert this number into a `BigDecimal`.
    ///
    /// [`BigDecimal`] does not distinguish between negative zero and
    /// positive zero. Calling code is responsible for remembering
    /// whether this `Number` was negative zero or positive zero.
    ///
    /// # Examples
    ///
    /// Positive and negative zeros become the same [`BigDecimal`]:
    ///
    /// ```rust,ignore
    /// use bigdecimal::BigDecimal
    ///
    /// assert_eq!(Number::MinusZeroInt.into_big_decimal(), BigDecimal::zero());
    /// assert_eq!(Number::MinusZeroFloat.into_big_decimal(), BigDecimal::zero());
    /// ```
    pub fn into_big_decimal(self) -> BigDecimal {
        match self {
            Number::MinusZeroInt => -BigDecimal::zero(),
            Number::MinusZeroFloat => -BigDecimal::zero(),
            Number::BigInt(n) => BigDecimal::from(n),
            Number::BigDecimal(n) => n,
        }
    }

    /// Number of characters needed to print the integral part of the number.
    ///
    /// The number of characters includes one character to represent the
    /// minus sign ("-") if this number is negative.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use num_bigint::{BigInt, Sign};
    ///
    /// assert_eq!(
    ///     Number::BigInt(BigInt::new(Sign::Plus, vec![123])).num_digits(),
    ///     3
    /// );
    /// assert_eq!(
    ///     Number::BigInt(BigInt::new(Sign::Minus, vec![123])).num_digits(),
    ///     4
    /// );
    /// assert_eq!(Number::BigDecimal("123.45".parse()).num_digits(), 3);
    /// assert_eq!(Number::MinusZeroInt.num_digits(), 2);
    /// ```
    pub fn num_digits(&self) -> usize {
        match self {
            Number::MinusZeroInt => 2,
            Number::MinusZeroFloat => 2,
            Number::BigInt(n) => n.to_string().len(),
            Number::BigDecimal(n) => {
                let s = format!("{}", n);
                s.find('.').unwrap_or_else(|| s.len())
            }
        }
    }
}

/// Parse a [`BigInt`] or a "negative zero" from a string.
///
/// Even though "negative zero" is usually only a concept for floating
/// point numbers, the `seq` tool allows negative zero at the start of a
/// sequence.
fn parse_bigint_or_minus_zero(s: &str) -> Result<Number, ParseBigIntError> {
    // If `s` is '-0', then `parse()` returns `BigInt::zero()`, but we
    // need to return `Number::MinusZeroInt` instead.
    s.parse::<BigInt>().map(|n| {
        if n == BigInt::zero() && s.starts_with('-') {
            Number::MinusZeroInt
        } else {
            Number::BigInt(n)
        }
    })
}

/// Parse a [`BigDecimal`] or a negative zero from a string.
fn parse_bigdecimal_or_minus_zero(s: &str) -> Result<Number, ParseBigDecimalError> {
    // If `s` is '-0.0', then `parse()` returns `BigDecimal::zero()`,
    // but we need to return `Number::MinusZeroFloat` instead.
    s.parse::<BigDecimal>().map(|x| {
        if x == BigDecimal::zero() && s.starts_with('-') {
            Number::MinusZeroFloat
        } else {
            Number::BigDecimal(x)
        }
    })
}

/// The error message when an argument is a NaN.
fn nan_error_message(arg: &str) -> String {
    format!(
        "invalid 'not-a-number' argument: {}\nTry '{} --help' for more information.",
        arg.quote(),
        uucore::execution_phrase(),
    )
}

/// The error message when an argument is not a valid floating point number.
fn float_error_message(arg: &str) -> String {
    format!(
        "invalid floating point argument: {}\nTry '{} --help' for more information.",
        arg.quote(),
        uucore::execution_phrase(),
    )
}

impl FromStr for Number {
    type Err = String;

    fn from_str(mut s: &str) -> Result<Self, Self::Err> {
        if s.starts_with('+') {
            s = &s[1..];
        }

        // TODO Add support for positive and negative infinity; the
        // `bigdecimal` crate does not support them:
        // https://github.com/akubera/bigdecimal-rs/issues/67
        parse_bigint_or_minus_zero(s).or_else(|_| {
            parse_bigdecimal_or_minus_zero(s).map_err(|_| match s.parse::<f64>() {
                Ok(x) if x.is_nan() => nan_error_message(s),
                // The `Ok(_)` pattern should never match, because
                // any floating point number should be parsed by the
                // `parse::<BigDecimal>()` call above.
                Ok(_) | Err(_) => float_error_message(s),
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::Number;
    use num_bigint::{BigInt, Sign};

    #[test]
    fn test_number_num_digits() {
        assert_eq!(
            Number::BigInt(BigInt::new(Sign::Plus, vec![123])).num_digits(),
            3
        );
        assert_eq!(
            Number::BigInt(BigInt::new(Sign::Minus, vec![123])).num_digits(),
            4
        );
        assert_eq!(
            Number::BigDecimal("123.45".parse().unwrap()).num_digits(),
            3
        );
        assert_eq!(Number::BigDecimal("1000".parse().unwrap()).num_digits(), 4);
        assert_eq!(Number::MinusZeroInt.num_digits(), 2);
        assert_eq!("1e3".parse::<Number>().unwrap().num_digits(), 4);
        assert_eq!("1000".parse::<Number>().unwrap().num_digits(), 4);
        assert_eq!("1000.1".parse::<Number>().unwrap().num_digits(), 4);
    }
}
