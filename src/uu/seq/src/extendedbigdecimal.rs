// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
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
use std::ops::{Add, Neg};

use bigdecimal::BigDecimal;
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
}

impl ExtendedBigDecimal {
    #[cfg(test)]
    pub fn zero() -> Self {
        Self::BigDecimal(0.into())
    }

    pub fn one() -> Self {
        Self::BigDecimal(1.into())
    }

    pub fn from_f128(value: f128) -> Self {
        // this code is adapted from num_bigint::BigDecimal::from_f64, but without the fast path for
        // subnormal f128s and all in one function

        let (neg, pow, frac) = match value.classify() {
            std::num::FpCategory::Nan => return ExtendedBigDecimal::Nan,
            std::num::FpCategory::Infinite => {
                return if value.is_sign_negative() {
                    ExtendedBigDecimal::MinusInfinity
                } else {
                    ExtendedBigDecimal::Infinity
                };
            }
            std::num::FpCategory::Zero => {
                return if value.is_sign_negative() {
                    ExtendedBigDecimal::MinusZero
                } else {
                    ExtendedBigDecimal::zero()
                };
            }
            std::num::FpCategory::Subnormal | std::num::FpCategory::Normal => {
                /// f128::MANTISSA_DIGITS is 113 (because of the leading 1 in normal floats, but it
                /// actually only has 112-bits)
                const MANTISSA_BITS: u32 = f128::MANTISSA_DIGITS - 1;
                /// The value of the leading one
                const MANTISSA_LEADING_ONE: u128 = 1 << MANTISSA_BITS;
                /// A mask that is all ones for the matissa bits
                const MANTISSA_MASK: u128 = MANTISSA_LEADING_ONE - 1;
                /// 15-bit exponent
                const EXPONENT_MASK: u128 = (1 << 15) - 1;

                let bits = value.to_bits();

                // extract mantissa (mask out the rest of the bits and add the leading one)
                let frac = (bits & MANTISSA_MASK)
                    + if value.is_normal() {
                        MANTISSA_LEADING_ONE
                    } else {
                        0
                    };

                // extract exponent (remove mantissa then mask out the rest of the bits (sign bit))
                let exp = (bits >> MANTISSA_BITS) & EXPONENT_MASK;

                // convert exponent to a power of two (subtract bias and size of mantissa)
                let pow = exp as i64 - 16383 - i64::from(MANTISSA_BITS);

                (value.is_sign_negative(), pow, frac)
            }
        };
        let (frac, scale) = match pow.cmp(&0) {
            Ordering::Less => {
                let trailing_zeros = std::cmp::min(frac.trailing_zeros(), -pow as u32);

                // Reduce fraction by removing common factors
                let reduced_frac = frac >> trailing_zeros;
                let reduced_pow = pow + trailing_zeros as i64;

                // We need to scale up by 5^reduced_pow as `scale` is 10^scale instead of 2^scale
                // (and 10^scale = 5^scale * 2^scale)
                (
                    reduced_frac * num_bigint::BigUint::from(5u8).pow(-reduced_pow as u32),
                    // scale is positive if the power is negative, so flip the sign
                    -reduced_pow,
                )
            }
            Ordering::Equal => (num_bigint::BigUint::from(frac), 0),
            Ordering::Greater => (frac * num_bigint::BigUint::from(2u32).pow(pow as u32), 0),
        };

        ExtendedBigDecimal::BigDecimal(BigDecimal::new(
            num_bigint::BigInt::from_biguint(
                if neg {
                    num_bigint::Sign::Minus
                } else {
                    num_bigint::Sign::Plus
                },
                frac,
            ),
            scale,
        ))
    }

    pub fn to_f128(&self) -> f128 {
        match self {
            ExtendedBigDecimal::Infinity => f128::INFINITY,
            ExtendedBigDecimal::MinusInfinity => f128::NEG_INFINITY,
            ExtendedBigDecimal::MinusZero => -0.0f128,
            ExtendedBigDecimal::Nan => f128::NAN,
            // Adapted from <BigDecimal as ToPrimitive>::to_f64
            ExtendedBigDecimal::BigDecimal(n) => {
                // Descruture BigDecimal
                let (n, e) = n.as_bigint_and_exponent();
                let bits = n.bits();
                let (sign, digits) = n.to_u64_digits();

                // Extract most significant digits (we truncate the rest as they don't affect the
                // conversion to f128):
                //
                // digits are stores in reverse order (e.g. 1u128 << 64 = [0, 1])
                let (mantissa, exponent) = match digits[..] {
                    // Last two digits
                    [.., a, b] => {
                        let m = (u128::from(b) << 64) + u128::from(a);

                        // Strip mantissa digits from the exponent:
                        //
                        // Assume mantissa = 0b0...0110000 truncated rest
                        //                     ^...^^^^^^^                         (size = u128::BITS)
                        //                     ^...^^^                    mantissa
                        //                            ^^^^                         (size = mantissa.trailing_zeros())
                        //                            ^^^^ ^^^^^...^^^^^^ exponent
                        //                     ^...^^^^^^^ ^^^^^...^^^^^^          (size = bits)
                        // u128::BITS - mantissa.trailing_zeros() = bits(mantissa)
                        // bits - bits(mantissa) = exponenet
                        let e = bits - u64::from(u128::BITS - m.trailing_zeros());
                        // FIXME: something is wrong here
                        (m >> m.trailing_zeros(), e)
                    }
                    // Single digit
                    // FIXME: something is wrong here
                    [a] => (
                        u128::from(a) >> a.trailing_zeros(),
                        a.trailing_zeros().into(),
                    ),
                    // Zero (fast path)
                    [] => return 0.0,
                };

                // Convert to f128
                let val = if exponent > f128::MAX_EXP as u64 {
                    f128::INFINITY
                } else {
                    // matissa * 2^exponent * 10^(-e)
                    //                        ^^^^^^^ big decimal exponent
                    // ^^^^^^^^^^^^^^^^^^^^           big uint to f128
                    (mantissa as f128)
                        * f128::powi(2.0, exponent as i32)
                        * f128::powi(10.0, e.neg() as i32)
                };

                // Set sign
                if matches!(sign, num_bigint::Sign::Minus) {
                    -val
                } else {
                    val
                }
            }
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
