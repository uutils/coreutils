//! Counting number of digits needed to represent a number.
//!
//! The [`num_integral_digits`] and [`num_fractional_digits`] functions
//! count the number of digits needed to represent a number in decimal
//! notation (like "123.456").
use std::convert::TryInto;
use std::num::ParseIntError;

use uucore::display::Quotable;

/// The number of digits after the decimal point in a given number.
///
/// The input `s` is a string representing a number, either an integer
/// or a floating point number in either decimal notation or scientific
/// notation. This function returns the number of digits after the
/// decimal point needed to print the number in decimal notation.
///
/// # Examples
///
/// ```rust,ignore
/// assert_eq!(num_fractional_digits("123.45e-1").unwrap(), 3);
/// ```
pub fn num_fractional_digits(s: &str) -> Result<usize, ParseIntError> {
    match (s.find('.'), s.find('e')) {
        // For example, "123456".
        (None, None) => Ok(0),

        // For example, "123e456".
        (None, Some(j)) => {
            let exponent: i64 = s[j + 1..].parse()?;
            if exponent < 0 {
                Ok(-exponent as usize)
            } else {
                Ok(0)
            }
        }

        // For example, "123.456".
        (Some(i), None) => Ok(s.len() - (i + 1)),

        // For example, "123.456e789".
        (Some(i), Some(j)) if i < j => {
            // Because of the match guard, this subtraction will not underflow.
            let num_digits_between_decimal_point_and_e = (j - (i + 1)) as i64;
            let exponent: i64 = s[j + 1..].parse()?;
            if num_digits_between_decimal_point_and_e < exponent {
                Ok(0)
            } else {
                Ok((num_digits_between_decimal_point_and_e - exponent)
                    .try_into()
                    .unwrap())
            }
        }
        _ => crash!(
            1,
            "invalid floating point argument: {}\n Try '{} --help' for more information.",
            s.quote(),
            uucore::execution_phrase()
        ),
    }
}

/// The number of digits before the decimal point in a given number.
///
/// The input `s` is a string representing a number, either an integer
/// or a floating point number in either decimal notation or scientific
/// notation. This function returns the number of digits before the
/// decimal point needed to print the number in decimal notation.
///
/// # Examples
///
/// ```rust,ignore
/// assert_eq!(num_fractional_digits("123.45e-1").unwrap(), 2);
/// ```
pub fn num_integral_digits(s: &str) -> Result<usize, ParseIntError> {
    match (s.find('.'), s.find('e')) {
        // For example, "123456".
        (None, None) => Ok(s.len()),

        // For example, "123e456".
        (None, Some(j)) => {
            let exponent: i64 = s[j + 1..].parse()?;
            let total = j as i64 + exponent;
            if total < 1 {
                Ok(1)
            } else {
                Ok(total.try_into().unwrap())
            }
        }

        // For example, "123.456".
        (Some(i), None) => Ok(i),

        // For example, "123.456e789".
        (Some(i), Some(j)) => {
            let exponent: i64 = s[j + 1..].parse()?;
            let minimum: usize = {
                let integral_part: f64 = crash_if_err!(1, s[..j].parse());
                if integral_part == -0.0 && integral_part.is_sign_negative() {
                    2
                } else {
                    1
                }
            };

            let total = i as i64 + exponent;
            if total < minimum as i64 {
                Ok(minimum)
            } else {
                Ok(total.try_into().unwrap())
            }
        }
    }
}

#[cfg(test)]
mod tests {

    mod test_num_integral_digits {
        use crate::num_integral_digits;

        #[test]
        fn test_integer() {
            assert_eq!(num_integral_digits("123").unwrap(), 3);
        }

        #[test]
        fn test_decimal() {
            assert_eq!(num_integral_digits("123.45").unwrap(), 3);
        }

        #[test]
        fn test_scientific_no_decimal_positive_exponent() {
            assert_eq!(num_integral_digits("123e4").unwrap(), 3 + 4);
        }

        #[test]
        fn test_scientific_with_decimal_positive_exponent() {
            assert_eq!(num_integral_digits("123.45e6").unwrap(), 3 + 6);
        }

        #[test]
        fn test_scientific_no_decimal_negative_exponent() {
            assert_eq!(num_integral_digits("123e-4").unwrap(), 1);
        }

        #[test]
        fn test_scientific_with_decimal_negative_exponent() {
            assert_eq!(num_integral_digits("123.45e-6").unwrap(), 1);
            assert_eq!(num_integral_digits("123.45e-1").unwrap(), 2);
        }
    }

    mod test_num_fractional_digits {
        use crate::num_fractional_digits;

        #[test]
        fn test_integer() {
            assert_eq!(num_fractional_digits("123").unwrap(), 0);
        }

        #[test]
        fn test_decimal() {
            assert_eq!(num_fractional_digits("123.45").unwrap(), 2);
        }

        #[test]
        fn test_scientific_no_decimal_positive_exponent() {
            assert_eq!(num_fractional_digits("123e4").unwrap(), 0);
        }

        #[test]
        fn test_scientific_with_decimal_positive_exponent() {
            assert_eq!(num_fractional_digits("123.45e6").unwrap(), 0);
            assert_eq!(num_fractional_digits("123.45e1").unwrap(), 1);
        }

        #[test]
        fn test_scientific_no_decimal_negative_exponent() {
            assert_eq!(num_fractional_digits("123e-4").unwrap(), 4);
            assert_eq!(num_fractional_digits("123e-1").unwrap(), 1);
        }

        #[test]
        fn test_scientific_with_decimal_negative_exponent() {
            assert_eq!(num_fractional_digits("123.45e-6").unwrap(), 8);
            assert_eq!(num_fractional_digits("123.45e-1").unwrap(), 3);
        }
    }
}
