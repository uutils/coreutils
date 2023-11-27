// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore extendedbigdecimal
use num_traits::Zero;

use crate::extendedbigdecimal::ExtendedBigDecimal;

/// A number with a specified number of integer and fractional digits.
///
/// This struct can be used to represent a number along with information
/// on how many significant digits to use when displaying the number.
/// The [`PreciseNumber::num_integral_digits`] field also includes the width needed to
/// display the "-" character for a negative number.
///
/// You can get an instance of this struct by calling [`str::parse`].
#[derive(Debug)]
pub struct PreciseNumber {
    pub number: ExtendedBigDecimal,
    pub num_integral_digits: usize,
    pub num_fractional_digits: usize,
}

impl PreciseNumber {
    pub fn new(
        number: ExtendedBigDecimal,
        num_integral_digits: usize,
        num_fractional_digits: usize,
    ) -> Self {
        Self {
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
        Self::new(ExtendedBigDecimal::one(), 1, 0)
    }

    /// Decide whether this number is zero (either positive or negative).
    pub fn is_zero(&self) -> bool {
        // We would like to implement `num_traits::Zero`, but it
        // requires an addition implementation, and we don't want to
        // implement that here.
        self.number.is_zero()
    }
}
