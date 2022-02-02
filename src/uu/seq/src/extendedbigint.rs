// spell-checker:ignore bigint extendedbigint extendedbigdecimal
//! An arbitrary precision integer that can also represent infinity, NaN, etc.
//!
//! Usually infinity, NaN, and negative zero are only represented for
//! floating point numbers. The [`ExtendedBigInt`] enumeration provides
//! a representation of those things with the set of integers. The
//! finite values are stored as [`BigInt`] instances.
//!
//! # Examples
//!
//! Addition works for [`ExtendedBigInt`] as it does for floats. For
//! example, adding infinity to any finite value results in infinity:
//!
//! ```rust,ignore
//! let summand1 = ExtendedBigInt::BigInt(BigInt::zero());
//! let summand2 = ExtendedBigInt::Infinity;
//! assert_eq!(summand1 + summand2, ExtendedBigInt::Infinity);
//! ```
use std::cmp::Ordering;
use std::fmt::Display;
use std::ops::Add;

use num_bigint::BigInt;
use num_bigint::ToBigInt;
use num_traits::One;
use num_traits::Zero;

use crate::extendedbigdecimal::ExtendedBigDecimal;

#[derive(Debug, Clone)]
pub enum ExtendedBigInt {
    BigInt(BigInt),
    Infinity,
    MinusInfinity,
    MinusZero,
    Nan,
}

impl ExtendedBigInt {
    /// The integer number one.
    pub fn one() -> Self {
        // We would like to implement `num_traits::One`, but it requires
        // a multiplication implementation, and we don't want to
        // implement that here.
        Self::BigInt(BigInt::one())
    }
}

impl From<ExtendedBigDecimal> for ExtendedBigInt {
    fn from(big_decimal: ExtendedBigDecimal) -> Self {
        match big_decimal {
            // TODO When can this fail?
            ExtendedBigDecimal::BigDecimal(x) => Self::BigInt(x.to_bigint().unwrap()),
            ExtendedBigDecimal::Infinity => Self::Infinity,
            ExtendedBigDecimal::MinusInfinity => Self::MinusInfinity,
            ExtendedBigDecimal::MinusZero => Self::MinusZero,
            ExtendedBigDecimal::Nan => Self::Nan,
        }
    }
}

impl Display for ExtendedBigInt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BigInt(n) => n.fmt(f),
            Self::Infinity => f32::INFINITY.fmt(f),
            Self::MinusInfinity => f32::NEG_INFINITY.fmt(f),
            Self::MinusZero => {
                // FIXME Come up with a way of formatting this with a
                // "-" prefix.
                0.fmt(f)
            }
            Self::Nan => "nan".fmt(f),
        }
    }
}

impl Zero for ExtendedBigInt {
    fn zero() -> Self {
        Self::BigInt(BigInt::zero())
    }
    fn is_zero(&self) -> bool {
        match self {
            Self::BigInt(n) => n.is_zero(),
            Self::MinusZero => true,
            _ => false,
        }
    }
}

impl Add for ExtendedBigInt {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        match (self, other) {
            (Self::BigInt(m), Self::BigInt(n)) => Self::BigInt(m.add(n)),
            (Self::BigInt(_), Self::MinusInfinity) => Self::MinusInfinity,
            (Self::BigInt(_), Self::Infinity) => Self::Infinity,
            (Self::BigInt(_), Self::Nan) => Self::Nan,
            (Self::BigInt(m), Self::MinusZero) => Self::BigInt(m),
            (Self::Infinity, Self::BigInt(_)) => Self::Infinity,
            (Self::Infinity, Self::Infinity) => Self::Infinity,
            (Self::Infinity, Self::MinusZero) => Self::Infinity,
            (Self::Infinity, Self::MinusInfinity) => Self::Nan,
            (Self::Infinity, Self::Nan) => Self::Nan,
            (Self::MinusInfinity, Self::BigInt(_)) => Self::MinusInfinity,
            (Self::MinusInfinity, Self::MinusInfinity) => Self::MinusInfinity,
            (Self::MinusInfinity, Self::MinusZero) => Self::MinusInfinity,
            (Self::MinusInfinity, Self::Infinity) => Self::Nan,
            (Self::MinusInfinity, Self::Nan) => Self::Nan,
            (Self::Nan, _) => Self::Nan,
            (Self::MinusZero, other) => other,
        }
    }
}

impl PartialEq for ExtendedBigInt {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::BigInt(m), Self::BigInt(n)) => m.eq(n),
            (Self::BigInt(_), Self::MinusInfinity) => false,
            (Self::BigInt(_), Self::Infinity) => false,
            (Self::BigInt(_), Self::Nan) => false,
            (Self::BigInt(_), Self::MinusZero) => false,
            (Self::Infinity, Self::BigInt(_)) => false,
            (Self::Infinity, Self::Infinity) => true,
            (Self::Infinity, Self::MinusZero) => false,
            (Self::Infinity, Self::MinusInfinity) => false,
            (Self::Infinity, Self::Nan) => false,
            (Self::MinusInfinity, Self::BigInt(_)) => false,
            (Self::MinusInfinity, Self::Infinity) => false,
            (Self::MinusInfinity, Self::MinusZero) => false,
            (Self::MinusInfinity, Self::MinusInfinity) => true,
            (Self::MinusInfinity, Self::Nan) => false,
            (Self::Nan, _) => false,
            (Self::MinusZero, Self::BigInt(_)) => false,
            (Self::MinusZero, Self::Infinity) => false,
            (Self::MinusZero, Self::MinusZero) => true,
            (Self::MinusZero, Self::MinusInfinity) => false,
            (Self::MinusZero, Self::Nan) => false,
        }
    }
}

impl PartialOrd for ExtendedBigInt {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (Self::BigInt(m), Self::BigInt(n)) => m.partial_cmp(n),
            (Self::BigInt(_), Self::MinusInfinity) => Some(Ordering::Greater),
            (Self::BigInt(_), Self::Infinity) => Some(Ordering::Less),
            (Self::BigInt(_), Self::Nan) => None,
            (Self::BigInt(m), Self::MinusZero) => m.partial_cmp(&BigInt::zero()),
            (Self::Infinity, Self::BigInt(_)) => Some(Ordering::Greater),
            (Self::Infinity, Self::Infinity) => Some(Ordering::Equal),
            (Self::Infinity, Self::MinusZero) => Some(Ordering::Greater),
            (Self::Infinity, Self::MinusInfinity) => Some(Ordering::Greater),
            (Self::Infinity, Self::Nan) => None,
            (Self::MinusInfinity, Self::BigInt(_)) => Some(Ordering::Less),
            (Self::MinusInfinity, Self::Infinity) => Some(Ordering::Less),
            (Self::MinusInfinity, Self::MinusZero) => Some(Ordering::Less),
            (Self::MinusInfinity, Self::MinusInfinity) => Some(Ordering::Equal),
            (Self::MinusInfinity, Self::Nan) => None,
            (Self::Nan, _) => None,
            (Self::MinusZero, Self::BigInt(n)) => BigInt::zero().partial_cmp(n),
            (Self::MinusZero, Self::Infinity) => Some(Ordering::Less),
            (Self::MinusZero, Self::MinusZero) => Some(Ordering::Equal),
            (Self::MinusZero, Self::MinusInfinity) => Some(Ordering::Greater),
            (Self::MinusZero, Self::Nan) => None,
        }
    }
}

#[cfg(test)]
mod tests {

    use num_bigint::BigInt;
    use num_traits::Zero;

    use crate::extendedbigint::ExtendedBigInt;

    #[test]
    fn test_addition_infinity() {
        let summand1 = ExtendedBigInt::BigInt(BigInt::zero());
        let summand2 = ExtendedBigInt::Infinity;
        assert_eq!(summand1 + summand2, ExtendedBigInt::Infinity);
    }

    #[test]
    fn test_addition_minus_infinity() {
        let summand1 = ExtendedBigInt::BigInt(BigInt::zero());
        let summand2 = ExtendedBigInt::MinusInfinity;
        assert_eq!(summand1 + summand2, ExtendedBigInt::MinusInfinity);
    }

    #[test]
    fn test_addition_nan() {
        let summand1 = ExtendedBigInt::BigInt(BigInt::zero());
        let summand2 = ExtendedBigInt::Nan;
        let sum = summand1 + summand2;
        match sum {
            ExtendedBigInt::Nan => (),
            _ => unreachable!(),
        }
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", ExtendedBigInt::BigInt(BigInt::zero())), "0");
        assert_eq!(format!("{}", ExtendedBigInt::Infinity), "inf");
        assert_eq!(format!("{}", ExtendedBigInt::MinusInfinity), "-inf");
        assert_eq!(format!("{}", ExtendedBigInt::Nan), "nan");
        // FIXME Come up with a way of displaying negative zero as
        // "-0". Currently it displays as just "0".
        //
        //     assert_eq!(format!("{}", ExtendedBigInt::MinusZero), "-0");
        //
    }
}
