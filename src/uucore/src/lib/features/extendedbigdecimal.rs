// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore bigdecimal extendedbigdecimal biguint
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
use std::ops::Add;
use std::ops::Neg;

use bigdecimal::BigDecimal;
use bigdecimal::num_bigint::BigUint;
use num_traits::FromPrimitive;
use num_traits::Signed;
use num_traits::Zero;

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

    /// Floating point negative NaN.
    ///
    /// This is represented as its own enumeration member instead of as
    /// a [`BigDecimal`] because the `bigdecimal` library does not
    /// support NaN, see [here][0].
    ///
    /// [0]: https://github.com/akubera/bigdecimal-rs/issues/67
    MinusNan,
}

impl From<f64> for ExtendedBigDecimal {
    fn from(val: f64) -> Self {
        if val.is_nan() {
            if val.is_sign_negative() {
                ExtendedBigDecimal::MinusNan
            } else {
                ExtendedBigDecimal::Nan
            }
        } else if val.is_infinite() {
            if val.is_sign_negative() {
                ExtendedBigDecimal::MinusInfinity
            } else {
                ExtendedBigDecimal::Infinity
            }
        } else if val.is_zero() && val.is_sign_negative() {
            ExtendedBigDecimal::MinusZero
        } else {
            ExtendedBigDecimal::BigDecimal(BigDecimal::from_f64(val).unwrap())
        }
    }
}

impl ExtendedBigDecimal {
    pub fn zero() -> Self {
        Self::BigDecimal(0.into())
    }

    pub fn one() -> Self {
        Self::BigDecimal(1.into())
    }

    pub fn to_biguint(&self) -> Option<BigUint> {
        match self {
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

impl Default for ExtendedBigDecimal {
    fn default() -> Self {
        Self::zero()
    }
}

impl Add for ExtendedBigDecimal {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        match (self, other) {
            (Self::BigDecimal(m), Self::BigDecimal(n)) => Self::BigDecimal(m.add(n)),
            (Self::BigDecimal(_), Self::MinusInfinity) => Self::MinusInfinity,
            (Self::BigDecimal(_), Self::Infinity) => Self::Infinity,
            (Self::BigDecimal(m), Self::MinusZero) => Self::BigDecimal(m),
            (Self::Infinity, Self::BigDecimal(_)) => Self::Infinity,
            (Self::Infinity, Self::Infinity) => Self::Infinity,
            (Self::Infinity, Self::MinusZero) => Self::Infinity,
            (Self::Infinity, Self::MinusInfinity) => Self::Nan,
            (Self::MinusInfinity, Self::BigDecimal(_)) => Self::MinusInfinity,
            (Self::MinusInfinity, Self::MinusInfinity) => Self::MinusInfinity,
            (Self::MinusInfinity, Self::MinusZero) => Self::MinusInfinity,
            (Self::MinusInfinity, Self::Infinity) => Self::Nan,
            (Self::Nan, _) => Self::Nan,
            (_, Self::Nan) => Self::Nan,
            (Self::MinusNan, _) => Self::MinusNan,
            (_, Self::MinusNan) => Self::MinusNan,
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
            (Self::BigDecimal(_), Self::MinusZero) => false,
            (Self::Infinity, Self::BigDecimal(_)) => false,
            (Self::Infinity, Self::Infinity) => true,
            (Self::Infinity, Self::MinusZero) => false,
            (Self::Infinity, Self::MinusInfinity) => false,
            (Self::MinusInfinity, Self::BigDecimal(_)) => false,
            (Self::MinusInfinity, Self::Infinity) => false,
            (Self::MinusInfinity, Self::MinusZero) => false,
            (Self::MinusInfinity, Self::MinusInfinity) => true,
            (Self::MinusZero, Self::BigDecimal(_)) => false,
            (Self::MinusZero, Self::Infinity) => false,
            (Self::MinusZero, Self::MinusZero) => true,
            (Self::MinusZero, Self::MinusInfinity) => false,
            (Self::Nan, _) => false,
            (Self::MinusNan, _) => false,
            (_, Self::Nan) => false,
            (_, Self::MinusNan) => false,
        }
    }
}

impl PartialOrd for ExtendedBigDecimal {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (Self::BigDecimal(m), Self::BigDecimal(n)) => m.partial_cmp(n),
            (Self::BigDecimal(_), Self::MinusInfinity) => Some(Ordering::Greater),
            (Self::BigDecimal(_), Self::Infinity) => Some(Ordering::Less),
            (Self::BigDecimal(m), Self::MinusZero) => m.partial_cmp(&BigDecimal::zero()),
            (Self::Infinity, Self::BigDecimal(_)) => Some(Ordering::Greater),
            (Self::Infinity, Self::Infinity) => Some(Ordering::Equal),
            (Self::Infinity, Self::MinusZero) => Some(Ordering::Greater),
            (Self::Infinity, Self::MinusInfinity) => Some(Ordering::Greater),
            (Self::MinusInfinity, Self::BigDecimal(_)) => Some(Ordering::Less),
            (Self::MinusInfinity, Self::Infinity) => Some(Ordering::Less),
            (Self::MinusInfinity, Self::MinusZero) => Some(Ordering::Less),
            (Self::MinusInfinity, Self::MinusInfinity) => Some(Ordering::Equal),
            (Self::MinusZero, Self::BigDecimal(n)) => BigDecimal::zero().partial_cmp(n),
            (Self::MinusZero, Self::Infinity) => Some(Ordering::Less),
            (Self::MinusZero, Self::MinusZero) => Some(Ordering::Equal),
            (Self::MinusZero, Self::MinusInfinity) => Some(Ordering::Greater),
            (Self::Nan, _) => None,
            (Self::MinusNan, _) => None,
            (_, Self::Nan) => None,
            (_, Self::MinusNan) => None,
        }
    }
}

impl Neg for ExtendedBigDecimal {
    type Output = Self;

    fn neg(self) -> Self::Output {
        match self {
            Self::BigDecimal(bd) => {
                if bd.is_zero() {
                    Self::MinusZero
                } else {
                    Self::BigDecimal(bd.neg())
                }
            }
            Self::MinusZero => Self::BigDecimal(BigDecimal::zero()),
            Self::Infinity => Self::MinusInfinity,
            Self::MinusInfinity => Self::Infinity,
            Self::Nan => Self::MinusNan,
            Self::MinusNan => Self::Nan,
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
}
