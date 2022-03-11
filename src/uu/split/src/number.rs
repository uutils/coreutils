//  * This file is part of the uutils coreutils package.
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.
// spell-checker:ignore zaaa zaab
//! A number in arbitrary radix expressed in a positional notation.
//!
//! Use the [`Number`] enum to represent an arbitrary number in an
//! arbitrary radix. A number can be incremented and can be
//! displayed. See the [`Number`] documentation for more information.
//!
//! See the Wikipedia articles on [radix] and [positional notation]
//! for more background information on those topics.
//!
//! [radix]: https://en.wikipedia.org/wiki/Radix
//! [positional notation]: https://en.wikipedia.org/wiki/Positional_notation
use std::error::Error;
use std::fmt::{self, Display, Formatter};

/// An overflow due to incrementing a number beyond its representable limit.
#[derive(Debug)]
pub struct Overflow;

impl fmt::Display for Overflow {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Overflow")
    }
}

impl Error for Overflow {}

/// A number in arbitrary radix expressed in a positional notation.
///
/// Use the [`Number`] enum to represent an arbitrary number in an
/// arbitrary radix. A number can be incremented with
/// [`Number::increment`].  The [`FixedWidthNumber`] overflows when
/// attempting to increment it beyond the maximum number that can be
/// represented in the specified width. The [`DynamicWidthNumber`]
/// follows a non-standard incrementing procedure that is used
/// specifically for the `split` program. See the
/// [`DynamicWidthNumber`] documentation for more information.
///
/// Numbers of radix
///
/// * 10 are displayable and rendered as decimal numbers (for example,
///   "00" or "917"),
/// * 16 are displayable and rendered as hexadecimal numbers (for example,
///   "00" or "e7f"),
/// * 26 are displayable and rendered as lowercase ASCII alphabetic
///   characters (for example, "aa" or "zax").
///
/// Numbers of other radices cannot be displayed. The display of a
/// [`DynamicWidthNumber`] includes a prefix whose length depends on
/// the width of the number. See the [`DynamicWidthNumber`]
/// documentation for more information.
///
/// The digits of a number are accessible via the [`Number::digits`]
/// method. The digits are represented as a [`Vec<u8>`] with the most
/// significant digit on the left and the least significant digit on
/// the right. Each digit is a nonnegative integer less than the
/// radix. For example, if the radix is 3, then `vec![1, 0, 2]`
/// represents the decimal number 11:
///
/// ```ignore
/// 1 * 3^2 + 0 * 3^1 + 2 * 3^0 = 9 + 0 + 2 = 11
/// ```
///
/// For the [`DynamicWidthNumber`], the digits are not unique in the
/// sense that repeatedly incrementing the number will eventually
/// yield `vec![0, 0]`, `vec![0, 0, 0], `vec![0, 0, 0, 0]`, etc.
/// That's okay because each of these numbers will be displayed
/// differently and we only intend to use these numbers for display
/// purposes and not for mathematical purposes.
#[derive(Clone)]
pub enum Number {
    /// A fixed-width representation of a number.
    FixedWidth(FixedWidthNumber),

    /// A representation of a number with a dynamically growing width.
    DynamicWidth(DynamicWidthNumber),
}

impl Number {
    /// The digits of this number in decreasing order of significance.
    ///
    /// The digits are represented as a [`Vec<u8>`] with the most
    /// significant digit on the left and the least significant digit
    /// on the right. Each digit is a nonnegative integer less than
    /// the radix. For example, if the radix is 3, then `vec![1, 0,
    /// 2]` represents the decimal number 11:
    ///
    /// ```ignore
    /// 1 * 3^2 + 0 * 3^1 + 2 * 3^0 = 9 + 0 + 2 = 11
    /// ```
    ///
    /// For the [`DynamicWidthNumber`], the digits are not unique in the
    /// sense that repeatedly incrementing the number will eventually
    /// yield `vec![0, 0]`, `vec![0, 0, 0], `vec![0, 0, 0, 0]`, etc.
    /// That's okay because each of these numbers will be displayed
    /// differently and we only intend to use these numbers for display
    /// purposes and not for mathematical purposes.
    #[allow(dead_code)]
    fn digits(&self) -> &Vec<u8> {
        match self {
            Self::FixedWidth(number) => &number.digits,
            Self::DynamicWidth(number) => &number.digits,
        }
    }

    /// Increment this number to its successor.
    ///
    /// If incrementing this number would result in an overflow beyond
    /// the maximum representable number, then return
    /// [`Err(Overflow)`]. The [`FixedWidthNumber`] overflows, but
    /// [`DynamicWidthNumber`] does not.
    ///
    /// The [`DynamicWidthNumber`] follows a non-standard incrementing
    /// procedure that is used specifically for the `split` program.
    /// See the [`DynamicWidthNumber`] documentation for more
    /// information.
    ///
    /// # Errors
    ///
    /// This method returns [`Err(Overflow)`] when attempting to
    /// increment beyond the largest representable number.
    ///
    /// # Examples
    ///
    /// Overflowing:
    ///
    /// ```rust,ignore
    ///
    /// use crate::number::FixedWidthNumber;
    /// use crate::number::Number;
    /// use crate::number::Overflow;
    ///
    /// // Radix 3, width of 1 digit.
    /// let mut number = Number::FixedWidth(FixedWidthNumber::new(3, 1));
    /// number.increment().unwrap();  // from 0 to 1
    /// number.increment().unwrap();  // from 1 to 2
    /// assert!(number.increment().is_err());
    /// ```
    pub fn increment(&mut self) -> Result<(), Overflow> {
        match self {
            Self::FixedWidth(number) => number.increment(),
            Self::DynamicWidth(number) => number.increment(),
        }
    }
}

impl Display for Number {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::FixedWidth(number) => number.fmt(f),
            Self::DynamicWidth(number) => number.fmt(f),
        }
    }
}

/// A positional notation representation of a fixed-width number.
///
/// The digits are represented as a [`Vec<u8>`] with the most
/// significant digit on the left and the least significant digit on
/// the right. Each digit is a nonnegative integer less than the
/// radix.
///
/// # Incrementing
///
/// This number starts at `vec![0; width]`, representing the number 0
/// width the specified number of digits. Incrementing this number
/// with [`Number::increment`] causes it to increase its value by 1 in
/// the usual sense. If the digits are `vec![radix - 1; width]`, then
/// an overflow would occur and the [`Number::increment`] method
/// returns an error.
///
/// # Displaying
///
/// This number is only displayable if `radix` is 10, 26, or 26. If
/// `radix` is 10 or 16, then the digits are concatenated and
/// displayed as a fixed-width decimal or hexadecimal number,
/// respectively. If `radix` is 26, then each digit is translated to
/// the corresponding lowercase ASCII alphabetic character (that is,
/// 'a', 'b', 'c', etc.) and concatenated.
#[derive(Clone)]
pub struct FixedWidthNumber {
    radix: u8,
    digits: Vec<u8>,
}

impl FixedWidthNumber {
    /// Instantiate a number of the given radix and width.
    pub fn new(radix: u8, width: usize) -> Self {
        Self {
            radix,
            digits: vec![0; width],
        }
    }

    /// Increment this number.
    ///
    /// This method adds one to this number. If incrementing this
    /// number would require more digits than are available with the
    /// specified width, then this method returns [`Err(Overflow)`].
    fn increment(&mut self) -> Result<(), Overflow> {
        for i in (0..self.digits.len()).rev() {
            // Increment the current digit.
            self.digits[i] += 1;

            // If the digit overflows, then set it to 0 and continue
            // to the next iteration to increment the next most
            // significant digit. Otherwise, terminate the loop, since
            // there will be no further changes to any higher order
            // digits.
            if self.digits[i] == self.radix {
                self.digits[i] = 0;
            } else {
                break;
            }
        }

        // Return an error on overflow, which is signified by all zeros.
        if self.digits == vec![0; self.digits.len()] {
            Err(Overflow)
        } else {
            Ok(())
        }
    }
}

impl Display for FixedWidthNumber {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self.radix {
            10 => {
                let digits: String = self.digits.iter().map(|d| (b'0' + d) as char).collect();
                write!(f, "{}", digits)
            }
            16 => {
                let digits: String = self
                    .digits
                    .iter()
                    .map(|d| (if *d < 10 { b'0' + d } else { b'a' + (d - 10) }) as char)
                    .collect();
                write!(f, "{}", digits)
            }
            26 => {
                let digits: String = self.digits.iter().map(|d| (b'a' + d) as char).collect();
                write!(f, "{}", digits)
            }
            _ => Err(fmt::Error),
        }
    }
}

/// A positional notation representation of a number of dynamically growing width.
///
/// The digits are represented as a [`Vec<u8>`] with the most
/// significant digit on the left and the least significant digit on
/// the right. Each digit is a nonnegative integer less than the
/// radix.
///
/// # Incrementing
///
/// This number starts at `vec![0, 0]`, representing the number 0 with
/// a width of 2 digits. Incrementing this number with
/// [`Number::increment`] causes it to increase its value by 1. When
/// incrementing the number would have caused it to change from
/// `vec![radix - 2, radix - 1]` to `vec![radix - 1, 0]`, it instead
/// increases its width by one and resets its value to 0. For example,
/// if the radix were 3, the digits were `vec![1, 2]`, and we called
/// [`Number::increment`], then the digits would become `vec![0, 0,
/// 0]`. In this way, the width grows by one each time the most
/// significant digit would have achieved its maximum value.
///
/// This notion of "incrementing" here does not match the notion of
/// incrementing the *value* of the number, it is just an abstract way
/// of updating the representation of the number in a way that is only
/// useful for the purposes of the `split` program.
///
/// # Displaying
///
/// This number is only displayable if `radix` is 10, 16, or 26. If
/// `radix` is 10 or 16, then the digits are concatenated and
/// displayed as a fixed-width decimal or hexadecimal number,
/// respectively, with a prefix of `n - 2` instances of the character
/// '9' of 'f', respectively, where `n` is the number of digits.  If
/// `radix` is 26, then each digit is translated to the corresponding
/// lowercase ASCII alphabetic character (that is, 'a', 'b', 'c',
/// etc.) and concatenated with a prefix of `n - 2` instances of the
/// character 'z'.
///
/// This notion of displaying the number is specific to the `split`
/// program.
#[derive(Clone)]
pub struct DynamicWidthNumber {
    radix: u8,
    digits: Vec<u8>,
}

impl DynamicWidthNumber {
    /// Instantiate a number of the given radix, starting with width 2.
    ///
    /// This associated function returns a new instance of the struct
    /// with the given radix and a width of two digits, both 0.
    pub fn new(radix: u8) -> Self {
        Self {
            radix,
            digits: vec![0, 0],
        }
    }

    /// Set all digits to zero.
    fn reset(&mut self) {
        for i in 0..self.digits.len() {
            self.digits[i] = 0;
        }
    }

    /// Increment this number.
    ///
    /// This method adds one to this number. The first time that the
    /// most significant digit would achieve its highest possible
    /// value (that is, `radix - 1`), then all the digits get reset to
    /// 0 and the number of digits increases by one.
    ///
    /// This method never returns an error.
    fn increment(&mut self) -> Result<(), Overflow> {
        for i in (0..self.digits.len()).rev() {
            // Increment the current digit.
            self.digits[i] += 1;

            // If the digit overflows, then set it to 0 and continue
            // to the next iteration to increment the next most
            // significant digit. Otherwise, terminate the loop, since
            // there will be no further changes to any higher order
            // digits.
            if self.digits[i] == self.radix {
                self.digits[i] = 0;
            } else {
                break;
            }
        }

        // If the most significant digit is at its maximum value, then
        // add another digit and reset all digits zero.
        if self.digits[0] == self.radix - 1 {
            self.digits.push(0);
            self.reset();
        }
        Ok(())
    }
}

impl Display for DynamicWidthNumber {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self.radix {
            10 => {
                let num_fill_chars = self.digits.len() - 2;
                let digits: String = self.digits.iter().map(|d| (b'0' + d) as char).collect();
                write!(
                    f,
                    "{empty:9<num_fill_chars$}{digits}",
                    empty = "",
                    num_fill_chars = num_fill_chars,
                    digits = digits,
                )
            }
            16 => {
                let num_fill_chars = self.digits.len() - 2;
                let digits: String = self
                    .digits
                    .iter()
                    .map(|d| (if *d < 10 { b'0' + d } else { b'a' + (d - 10) }) as char)
                    .collect();
                write!(
                    f,
                    "{empty:f<num_fill_chars$}{digits}",
                    empty = "",
                    num_fill_chars = num_fill_chars,
                    digits = digits,
                )
            }
            26 => {
                let num_fill_chars = self.digits.len() - 2;
                let digits: String = self.digits.iter().map(|d| (b'a' + d) as char).collect();
                write!(
                    f,
                    "{empty:z<num_fill_chars$}{digits}",
                    empty = "",
                    num_fill_chars = num_fill_chars,
                    digits = digits,
                )
            }
            _ => Err(fmt::Error),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::number::DynamicWidthNumber;
    use crate::number::FixedWidthNumber;
    use crate::number::Number;
    use crate::number::Overflow;

    #[test]
    fn test_dynamic_width_number_increment() {
        let mut n = Number::DynamicWidth(DynamicWidthNumber::new(3));
        assert_eq!(n.digits(), &vec![0, 0]);

        n.increment().unwrap();
        assert_eq!(n.digits(), &vec![0, 1]);

        n.increment().unwrap();
        assert_eq!(n.digits(), &vec![0, 2]);

        n.increment().unwrap();
        assert_eq!(n.digits(), &vec![1, 0]);

        n.increment().unwrap();
        assert_eq!(n.digits(), &vec![1, 1]);

        n.increment().unwrap();
        assert_eq!(n.digits(), &vec![1, 2]);

        n.increment().unwrap();
        assert_eq!(n.digits(), &vec![0, 0, 0]);

        n.increment().unwrap();
        assert_eq!(n.digits(), &vec![0, 0, 1]);
    }

    #[test]
    fn test_dynamic_width_number_display_alphabetic() {
        fn num(n: usize) -> Number {
            let mut number = Number::DynamicWidth(DynamicWidthNumber::new(26));
            for _ in 0..n {
                number.increment().unwrap();
            }
            number
        }

        assert_eq!(format!("{}", num(0)), "aa");
        assert_eq!(format!("{}", num(1)), "ab");
        assert_eq!(format!("{}", num(2)), "ac");
        assert_eq!(format!("{}", num(25)), "az");
        assert_eq!(format!("{}", num(26)), "ba");
        assert_eq!(format!("{}", num(27)), "bb");
        assert_eq!(format!("{}", num(28)), "bc");
        assert_eq!(format!("{}", num(26 + 25)), "bz");
        assert_eq!(format!("{}", num(26 + 26)), "ca");
        assert_eq!(format!("{}", num(26 * 25 - 1)), "yz");
        assert_eq!(format!("{}", num(26 * 25)), "zaaa");
        assert_eq!(format!("{}", num(26 * 25 + 1)), "zaab");
    }

    #[test]
    fn test_dynamic_width_number_display_numeric_decimal() {
        fn num(n: usize) -> Number {
            let mut number = Number::DynamicWidth(DynamicWidthNumber::new(10));
            for _ in 0..n {
                number.increment().unwrap();
            }
            number
        }

        assert_eq!(format!("{}", num(0)), "00");
        assert_eq!(format!("{}", num(9)), "09");
        assert_eq!(format!("{}", num(17)), "17");
        assert_eq!(format!("{}", num(10 * 9 - 1)), "89");
        assert_eq!(format!("{}", num(10 * 9)), "9000");
        assert_eq!(format!("{}", num(10 * 9 + 1)), "9001");
        assert_eq!(format!("{}", num(10 * 99 - 1)), "9899");
        assert_eq!(format!("{}", num(10 * 99)), "990000");
        assert_eq!(format!("{}", num(10 * 99 + 1)), "990001");
    }

    #[test]
    fn test_dynamic_width_number_display_numeric_hexadecimal() {
        fn num(n: usize) -> Number {
            let mut number = Number::DynamicWidth(DynamicWidthNumber::new(16));
            for _ in 0..n {
                number.increment().unwrap();
            }
            number
        }

        assert_eq!(format!("{}", num(0)), "00");
        assert_eq!(format!("{}", num(15)), "0f");
        assert_eq!(format!("{}", num(16)), "10");
        assert_eq!(format!("{}", num(17)), "11");
        assert_eq!(format!("{}", num(18)), "12");

        assert_eq!(format!("{}", num(16 * 15 - 1)), "ef");
        assert_eq!(format!("{}", num(16 * 15)), "f000");
        assert_eq!(format!("{}", num(16 * 15 + 1)), "f001");
        assert_eq!(format!("{}", num(16 * 255 - 1)), "feff");
        assert_eq!(format!("{}", num(16 * 255)), "ff0000");
        assert_eq!(format!("{}", num(16 * 255 + 1)), "ff0001");
    }

    #[test]
    fn test_fixed_width_number_increment() {
        let mut n = Number::FixedWidth(FixedWidthNumber::new(3, 2));
        assert_eq!(n.digits(), &vec![0, 0]);
        n.increment().unwrap();
        assert_eq!(n.digits(), &vec![0, 1]);
        n.increment().unwrap();
        assert_eq!(n.digits(), &vec![0, 2]);
        n.increment().unwrap();
        assert_eq!(n.digits(), &vec![1, 0]);
        n.increment().unwrap();
        assert_eq!(n.digits(), &vec![1, 1]);
        n.increment().unwrap();
        assert_eq!(n.digits(), &vec![1, 2]);
        n.increment().unwrap();
        assert_eq!(n.digits(), &vec![2, 0]);
        n.increment().unwrap();
        assert_eq!(n.digits(), &vec![2, 1]);
        n.increment().unwrap();
        assert_eq!(n.digits(), &vec![2, 2]);
        assert!(n.increment().is_err());
    }

    #[test]
    fn test_fixed_width_number_display_alphabetic() {
        fn num(n: usize) -> Result<Number, Overflow> {
            let mut number = Number::FixedWidth(FixedWidthNumber::new(26, 2));
            for _ in 0..n {
                number.increment()?;
            }
            Ok(number)
        }

        assert_eq!(format!("{}", num(0).unwrap()), "aa");
        assert_eq!(format!("{}", num(1).unwrap()), "ab");
        assert_eq!(format!("{}", num(2).unwrap()), "ac");
        assert_eq!(format!("{}", num(25).unwrap()), "az");
        assert_eq!(format!("{}", num(26).unwrap()), "ba");
        assert_eq!(format!("{}", num(27).unwrap()), "bb");
        assert_eq!(format!("{}", num(28).unwrap()), "bc");
        assert_eq!(format!("{}", num(26 + 25).unwrap()), "bz");
        assert_eq!(format!("{}", num(26 + 26).unwrap()), "ca");
        assert_eq!(format!("{}", num(26 * 25 - 1).unwrap()), "yz");
        assert_eq!(format!("{}", num(26 * 25).unwrap()), "za");
        assert_eq!(format!("{}", num(26 * 26 - 1).unwrap()), "zz");
        assert!(num(26 * 26).is_err());
    }

    #[test]
    fn test_fixed_width_number_display_numeric_decimal() {
        fn num(n: usize) -> Result<Number, Overflow> {
            let mut number = Number::FixedWidth(FixedWidthNumber::new(10, 2));
            for _ in 0..n {
                number.increment()?;
            }
            Ok(number)
        }

        assert_eq!(format!("{}", num(0).unwrap()), "00");
        assert_eq!(format!("{}", num(9).unwrap()), "09");
        assert_eq!(format!("{}", num(17).unwrap()), "17");
        assert_eq!(format!("{}", num(10 * 9 - 1).unwrap()), "89");
        assert_eq!(format!("{}", num(10 * 9).unwrap()), "90");
        assert_eq!(format!("{}", num(10 * 10 - 1).unwrap()), "99");
        assert!(num(10 * 10).is_err());
    }

    #[test]
    fn test_fixed_width_number_display_numeric_hexadecimal() {
        fn num(n: usize) -> Result<Number, Overflow> {
            let mut number = Number::FixedWidth(FixedWidthNumber::new(16, 2));
            for _ in 0..n {
                number.increment()?;
            }
            Ok(number)
        }

        assert_eq!(format!("{}", num(0).unwrap()), "00");
        assert_eq!(format!("{}", num(15).unwrap()), "0f");
        assert_eq!(format!("{}", num(17).unwrap()), "11");
        assert_eq!(format!("{}", num(16 * 15 - 1).unwrap()), "ef");
        assert_eq!(format!("{}", num(16 * 15).unwrap()), "f0");
        assert_eq!(format!("{}", num(16 * 16 - 1).unwrap()), "ff");
        assert!(num(16 * 16).is_err());
    }
}
