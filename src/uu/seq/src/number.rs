//! A type to represent the possible start, increment, and end values for seq.
//!
//! The [`Number`] enumeration represents the possible values for the
//! start, increment, and end values for `seq`. These may be integers,
//! floating point numbers, negative zero, etc. A [`Number`] can be
//! parsed from a string by calling [`parse`].
use std::str::FromStr;

use num_bigint::BigInt;
use num_traits::ToPrimitive;
use num_traits::Zero;

use uucore::display::Quotable;

/// An integral or floating point number.
pub enum Number {
    /// Negative zero, as if it were an integer.
    MinusZero,

    /// An arbitrary precision integer.
    BigInt(BigInt),

    /// A 64-bit float.
    F64(f64),
}

impl Number {
    /// Decide whether this number is zero (either positive or negative).
    pub fn is_zero(&self) -> bool {
        match self {
            Number::MinusZero => true,
            Number::BigInt(n) => n.is_zero(),
            Number::F64(n) => n.is_zero(),
        }
    }

    /// Convert this number into a `f64`.
    pub fn into_f64(self) -> f64 {
        match self {
            Number::MinusZero => -0.,
            // BigInt::to_f64() can not return None.
            Number::BigInt(n) => n.to_f64().unwrap(),
            Number::F64(n) => n,
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
    /// assert_eq!(Number::F64(123.45).num_digits(), 3);
    /// assert_eq!(Number::MinusZero.num_digits(), 2);
    /// ```
    pub fn num_digits(&self) -> usize {
        match self {
            Number::MinusZero => 2,
            Number::BigInt(n) => n.to_string().len(),
            Number::F64(n) => {
                let s = n.to_string();
                s.find('.').unwrap_or_else(|| s.len())
            }
        }
    }
}

impl FromStr for Number {
    type Err = String;
    fn from_str(mut s: &str) -> Result<Self, Self::Err> {
        if s.starts_with('+') {
            s = &s[1..];
        }

        match s.parse::<BigInt>() {
            Ok(n) => {
                // If `s` is '-0', then `parse()` returns
                // `BigInt::zero()`, but we need to return
                // `Number::MinusZero` instead.
                if n == BigInt::zero() && s.starts_with('-') {
                    Ok(Number::MinusZero)
                } else {
                    Ok(Number::BigInt(n))
                }
            }
            Err(_) => match s.parse::<f64>() {
                Ok(value) if value.is_nan() => Err(format!(
                    "invalid 'not-a-number' argument: {}\nTry '{} --help' for more information.",
                    s.quote(),
                    uucore::execution_phrase(),
                )),
                Ok(value) => Ok(Number::F64(value)),
                Err(_) => Err(format!(
                    "invalid floating point argument: {}\nTry '{} --help' for more information.",
                    s.quote(),
                    uucore::execution_phrase(),
                )),
            },
        }
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
        assert_eq!(Number::F64(123.45).num_digits(), 3);
        assert_eq!(Number::MinusZero.num_digits(), 2);
    }
}
