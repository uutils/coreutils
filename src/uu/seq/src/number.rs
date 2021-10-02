// spell-checker:ignore extendedbigdecimal extendedbigint
//! A type to represent the possible start, increment, and end values for seq.
//!
//! The [`Number`] enumeration represents the possible values for the
//! start, increment, and end values for `seq`. These may be integers,
//! floating point numbers, negative zero, etc. A [`Number`] can be
//! parsed from a string by calling [`parse`].
use num_traits::Zero;

use crate::extendedbigdecimal::ExtendedBigDecimal;
use crate::extendedbigint::ExtendedBigInt;

/// An integral or floating point number.
#[derive(Debug, PartialEq)]
pub enum Number {
    Int(ExtendedBigInt),
    Float(ExtendedBigDecimal),
}

impl Number {
    /// Decide whether this number is zero (either positive or negative).
    pub fn is_zero(&self) -> bool {
        // We would like to implement `num_traits::Zero`, but it
        // requires an addition implementation, and we don't want to
        // implement that here.
        match self {
            Number::Int(n) => n.is_zero(),
            Number::Float(x) => x.is_zero(),
        }
    }

    /// Convert this number into an `ExtendedBigDecimal`.
    pub fn into_extended_big_decimal(self) -> ExtendedBigDecimal {
        match self {
            Number::Int(n) => ExtendedBigDecimal::from(n),
            Number::Float(x) => x,
        }
    }

    /// Convert this number into a bigint, consuming it.
    ///
    /// For floats, this returns the [`ExtendedBigInt`] corresponding to
    /// the floor of the number.
    pub fn into_extended_big_int(self) -> ExtendedBigInt {
        match self {
            Number::Int(n) => n,
            Number::Float(x) => ExtendedBigInt::from(x),
        }
    }

    /// The integer number one.
    pub fn one() -> Self {
        // We would like to implement `num_traits::One`, but it requires
        // a multiplication implementation, and we don't want to
        // implement that here.
        Number::Int(ExtendedBigInt::one())
    }
}

/// A number with a specified number of integer and fractional digits.
///
/// This struct can be used to represent a number along with information
/// on how many significant digits to use when displaying the number.
/// The [`num_integral_digits`] field also includes the width needed to
/// display the "-" character for a negative number.
///
/// You can get an instance of this struct by calling [`str::parse`].
#[derive(Debug)]
pub struct PreciseNumber {
    pub number: Number,
    pub num_integral_digits: usize,
    pub num_fractional_digits: usize,
}

impl PreciseNumber {
    pub fn new(
        number: Number,
        num_integral_digits: usize,
        num_fractional_digits: usize,
    ) -> PreciseNumber {
        PreciseNumber {
            number,
            num_integral_digits,
            num_fractional_digits,
        }
    }

    /// The integer number one.
    pub fn one() -> Self {
        // We would like to implement `num_traits::One`, but it requires
        // a multiplication implementation, and we don't want to
        // implement that here.
        PreciseNumber::new(Number::one(), 1, 0)
    }

    /// Decide whether this number is zero (either positive or negative).
    pub fn is_zero(&self) -> bool {
        // We would like to implement `num_traits::Zero`, but it
        // requires an addition implementation, and we don't want to
        // implement that here.
        self.number.is_zero()
    }
}
