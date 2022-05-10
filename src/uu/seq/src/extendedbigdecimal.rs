// spell-checker:ignore bigdecimal extendedbigdecimal extendedbigint
//! An arbitrary precision float that can also represent infinity, NaN, etc.
//!
//! The finite values are stored as [`BigDecimal`] instances. Because
//! the `bigdecimal` library does not represent infinity, NaN, etc., we
//! need to represent them explicitly ourselves. The
//! [`ExtendedBigDecimal`] enumeration does that.
//!
//! # Examples
//!
//! Addition works for [`ExtendedBigDecimal`] as it does for floats. For
//! example, adding infinity to any finite value results in infinity:
//!
//! ```rust,ignore
//! let summand1 = ExtendedBigDecimal::BigDecimal(BigDecimal::zero());
//! let summand2 = ExtendedBigDecimal::Infinity;
//! assert_eq!(summand1 + summand2, ExtendedBigDecimal::Infinity);
//! ```
use std::cmp::Ordering;
use std::fmt::Display;
use std::ops::Add;

use bigdecimal::BigDecimal;
use num_bigint::BigInt;
use num_bigint::ToBigInt;
use num_traits::One;
use num_traits::Zero;

use crate::extendedbigint::ExtendedBigInt;

#[derive(Debug, Clone)]
pub enum ExtendedBigDecimal {
    /// Arbitrary precision floating point number.
    BigDecimal(BigDecimal),

    /// Floating point positive infinity.
    ///
    /// This is represented as its own enumeration member instead of as
    /// a [`BigDecimal`] because the `bigdecimal` library does not
    /// support infinity, see [here][0].
    ///
    /// [0]: https://github.com/akubera/bigdecimal-rs/issues/67
    Infinity,

    /// Floating point negative infinity.
    ///
    /// This is represented as its own enumeration member instead of as
    /// a [`BigDecimal`] because the `bigdecimal` library does not
    /// support infinity, see [here][0].
    ///
    /// [0]: https://github.com/akubera/bigdecimal-rs/issues/67
    MinusInfinity,

    /// Floating point negative zero.
    ///
    /// This is represented as its own enumeration member instead of as
    /// a [`BigDecimal`] because the `bigdecimal` library does not
    /// support negative zero.
    MinusZero,

    /// Floating point NaN.
    ///
    /// This is represented as its own enumeration member instead of as
    /// a [`BigDecimal`] because the `bigdecimal` library does not
    /// support NaN, see [here][0].
    ///
    /// [0]: https://github.com/akubera/bigdecimal-rs/issues/67
    Nan,
}

/// The smallest integer greater than or equal to this number.
fn ceil(x: BigDecimal) -> BigInt {
    if x.is_integer() {
        // Unwrapping the Option because it always returns Some
        x.to_bigint().unwrap()
    } else {
        (x + BigDecimal::one().half()).round(0).to_bigint().unwrap()
    }
}

/// The largest integer less than or equal to this number.
fn floor(x: BigDecimal) -> BigInt {
    if x.is_integer() {
        // Unwrapping the Option because it always returns Some
        x.to_bigint().unwrap()
    } else {
        (x - BigDecimal::one().half()).round(0).to_bigint().unwrap()
    }
}

impl ExtendedBigDecimal {
    /// The smallest integer greater than or equal to this number.
    pub fn ceil(self) -> ExtendedBigInt {
        match self {
            Self::BigDecimal(x) => ExtendedBigInt::BigInt(ceil(x)),
            other => From::from(other),
        }
    }

    /// The largest integer less than or equal to this number.
    pub fn floor(self) -> ExtendedBigInt {
        match self {
            Self::BigDecimal(x) => ExtendedBigInt::BigInt(floor(x)),
            other => From::from(other),
        }
    }
}

impl From<ExtendedBigInt> for ExtendedBigDecimal {
    fn from(big_int: ExtendedBigInt) -> Self {
        match big_int {
            ExtendedBigInt::BigInt(n) => Self::BigDecimal(BigDecimal::from(n)),
            ExtendedBigInt::Infinity => Self::Infinity,
            ExtendedBigInt::MinusInfinity => Self::MinusInfinity,
            ExtendedBigInt::MinusZero => Self::MinusZero,
            ExtendedBigInt::Nan => Self::Nan,
        }
    }
}

impl Display for ExtendedBigDecimal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BigDecimal(x) => {
                let (n, p) = x.as_bigint_and_exponent();
                match p {
                    0 => Self::BigDecimal(BigDecimal::new(n * 10, 1)).fmt(f),
                    _ => x.fmt(f),
                }
            }
            Self::Infinity => f32::INFINITY.fmt(f),
            Self::MinusInfinity => f32::NEG_INFINITY.fmt(f),
            Self::MinusZero => (-0.0f32).fmt(f),
            Self::Nan => "nan".fmt(f),
        }
    }
}

impl Zero for ExtendedBigDecimal {
    fn zero() -> Self {
        Self::BigDecimal(BigDecimal::zero())
    }
    fn is_zero(&self) -> bool {
        match self {
            Self::BigDecimal(n) => n.is_zero(),
            Self::MinusZero => true,
            _ => false,
        }
    }
}

impl Add for ExtendedBigDecimal {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        match (self, other) {
            (Self::BigDecimal(m), Self::BigDecimal(n)) => Self::BigDecimal(m.add(n)),
            (Self::BigDecimal(_), Self::MinusInfinity) => Self::MinusInfinity,
            (Self::BigDecimal(_), Self::Infinity) => Self::Infinity,
            (Self::BigDecimal(_), Self::Nan) => Self::Nan,
            (Self::BigDecimal(m), Self::MinusZero) => Self::BigDecimal(m),
            (Self::Infinity, Self::BigDecimal(_)) => Self::Infinity,
            (Self::Infinity, Self::Infinity) => Self::Infinity,
            (Self::Infinity, Self::MinusZero) => Self::Infinity,
            (Self::Infinity, Self::MinusInfinity) => Self::Nan,
            (Self::Infinity, Self::Nan) => Self::Nan,
            (Self::MinusInfinity, Self::BigDecimal(_)) => Self::MinusInfinity,
            (Self::MinusInfinity, Self::MinusInfinity) => Self::MinusInfinity,
            (Self::MinusInfinity, Self::MinusZero) => Self::MinusInfinity,
            (Self::MinusInfinity, Self::Infinity) => Self::Nan,
            (Self::MinusInfinity, Self::Nan) => Self::Nan,
            (Self::Nan, _) => Self::Nan,
            (Self::MinusZero, other) => other,
        }
    }
}

impl PartialEq for ExtendedBigDecimal {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::BigDecimal(m), Self::BigDecimal(n)) => m.eq(n),
            (Self::BigDecimal(_), Self::MinusInfinity) => false,
            (Self::BigDecimal(_), Self::Infinity) => false,
            (Self::BigDecimal(_), Self::Nan) => false,
            (Self::BigDecimal(_), Self::MinusZero) => false,
            (Self::Infinity, Self::BigDecimal(_)) => false,
            (Self::Infinity, Self::Infinity) => true,
            (Self::Infinity, Self::MinusZero) => false,
            (Self::Infinity, Self::MinusInfinity) => false,
            (Self::Infinity, Self::Nan) => false,
            (Self::MinusInfinity, Self::BigDecimal(_)) => false,
            (Self::MinusInfinity, Self::Infinity) => false,
            (Self::MinusInfinity, Self::MinusZero) => false,
            (Self::MinusInfinity, Self::MinusInfinity) => true,
            (Self::MinusInfinity, Self::Nan) => false,
            (Self::Nan, _) => false,
            (Self::MinusZero, Self::BigDecimal(_)) => false,
            (Self::MinusZero, Self::Infinity) => false,
            (Self::MinusZero, Self::MinusZero) => true,
            (Self::MinusZero, Self::MinusInfinity) => false,
            (Self::MinusZero, Self::Nan) => false,
        }
    }
}

impl PartialOrd for ExtendedBigDecimal {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (Self::BigDecimal(m), Self::BigDecimal(n)) => m.partial_cmp(n),
            (Self::BigDecimal(_), Self::MinusInfinity) => Some(Ordering::Greater),
            (Self::BigDecimal(_), Self::Infinity) => Some(Ordering::Less),
            (Self::BigDecimal(_), Self::Nan) => None,
            (Self::BigDecimal(m), Self::MinusZero) => m.partial_cmp(&BigDecimal::zero()),
            (Self::Infinity, Self::BigDecimal(_)) => Some(Ordering::Greater),
            (Self::Infinity, Self::Infinity) => Some(Ordering::Equal),
            (Self::Infinity, Self::MinusZero) => Some(Ordering::Greater),
            (Self::Infinity, Self::MinusInfinity) => Some(Ordering::Greater),
            (Self::Infinity, Self::Nan) => None,
            (Self::MinusInfinity, Self::BigDecimal(_)) => Some(Ordering::Less),
            (Self::MinusInfinity, Self::Infinity) => Some(Ordering::Less),
            (Self::MinusInfinity, Self::MinusZero) => Some(Ordering::Less),
            (Self::MinusInfinity, Self::MinusInfinity) => Some(Ordering::Equal),
            (Self::MinusInfinity, Self::Nan) => None,
            (Self::Nan, _) => None,
            (Self::MinusZero, Self::BigDecimal(n)) => BigDecimal::zero().partial_cmp(n),
            (Self::MinusZero, Self::Infinity) => Some(Ordering::Less),
            (Self::MinusZero, Self::MinusZero) => Some(Ordering::Equal),
            (Self::MinusZero, Self::MinusInfinity) => Some(Ordering::Greater),
            (Self::MinusZero, Self::Nan) => None,
        }
    }
}

#[cfg(test)]
mod tests {

    use bigdecimal::BigDecimal;
    use num_traits::Zero;

    use crate::extendedbigdecimal::ExtendedBigDecimal;

    #[test]
    fn test_addition_infinity() {
        let summand1 = ExtendedBigDecimal::BigDecimal(BigDecimal::zero());
        let summand2 = ExtendedBigDecimal::Infinity;
        assert_eq!(summand1 + summand2, ExtendedBigDecimal::Infinity);
    }

    #[test]
    fn test_addition_minus_infinity() {
        let summand1 = ExtendedBigDecimal::BigDecimal(BigDecimal::zero());
        let summand2 = ExtendedBigDecimal::MinusInfinity;
        assert_eq!(summand1 + summand2, ExtendedBigDecimal::MinusInfinity);
    }

    #[test]
    fn test_addition_nan() {
        let summand1 = ExtendedBigDecimal::BigDecimal(BigDecimal::zero());
        let summand2 = ExtendedBigDecimal::Nan;
        let sum = summand1 + summand2;
        match sum {
            ExtendedBigDecimal::Nan => (),
            _ => unreachable!(),
        }
    }

    #[test]
    fn test_display() {
        assert_eq!(
            format!("{}", ExtendedBigDecimal::BigDecimal(BigDecimal::zero())),
            "0.0"
        );
        assert_eq!(format!("{}", ExtendedBigDecimal::Infinity), "inf");
        assert_eq!(format!("{}", ExtendedBigDecimal::MinusInfinity), "-inf");
        assert_eq!(format!("{}", ExtendedBigDecimal::Nan), "nan");
        assert_eq!(format!("{}", ExtendedBigDecimal::MinusZero), "-0");
    }
}
