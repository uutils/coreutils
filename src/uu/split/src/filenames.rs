//  * This file is part of the uutils coreutils package.
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.
// spell-checker:ignore zaaa zaab
//! Compute filenames from a given index.
//!
//! The [`FilenameIterator`] yields filenames for use with ``split``.
//!
//! # Examples
//!
//! Create filenames of the form `chunk_??.txt`:
//!
//! ```rust,ignore
//! use crate::filenames::FilenameIterator;
//! use crate::filenames::SuffixType;
//!
//! let prefix = "chunk_".to_string();
//! let suffix = ".txt".to_string();
//! let width = 2;
//! let suffix_type = SuffixType::Alphabetic;
//! let it = FilenameIterator::new(prefix, suffix, width, suffix_type);
//!
//! assert_eq!(it.next().unwrap(), "chunk_aa.txt");
//! assert_eq!(it.next().unwrap(), "chunk_ab.txt");
//! assert_eq!(it.next().unwrap(), "chunk_ac.txt");
//! ```
use crate::number::DynamicWidthNumber;
use crate::number::FixedWidthNumber;
use crate::number::Number;

/// The format to use for suffixes in the filename for each output chunk.
#[derive(Clone, Copy)]
pub enum SuffixType {
    /// Lowercase ASCII alphabetic characters.
    Alphabetic,

    /// Decimal numbers.
    Decimal,

    /// Hexadecimal numbers.
    Hexadecimal,
}

impl SuffixType {
    /// The radix to use when representing the suffix string as digits.
    pub fn radix(&self) -> u8 {
        match self {
            Self::Alphabetic => 26,
            Self::Decimal => 10,
            Self::Hexadecimal => 16,
        }
    }
}

/// Compute filenames from a given index.
///
/// This iterator yields filenames for use with ``split``.
///
/// The `prefix` is prepended to each filename and the
/// `additional_suffix1 is appended to each filename.
///
/// If `suffix_length` is 0, then the variable portion of the filename
/// that identifies the current chunk will have a dynamically
/// increasing width. If `suffix_length` is greater than zero, then
/// the variable portion of the filename will always be exactly that
/// width in characters. In that case, after the iterator yields each
/// string of that width, the iterator is exhausted.
///
/// Finally, `suffix_type` controls which type of suffix to produce,
/// alphabetic or numeric.
///
/// # Examples
///
/// Create filenames of the form `chunk_??.txt`, where the `?`
/// characters are lowercase ASCII alphabetic characters:
///
/// ```rust,ignore
/// use crate::filenames::FilenameIterator;
/// use crate::filenames::SuffixType;
///
/// let prefix = "chunk_".to_string();
/// let suffix = ".txt".to_string();
/// let width = 2;
/// let suffix_type = SuffixType::Alphabetic;
/// let it = FilenameIterator::new(prefix, suffix, width, suffix_type);
///
/// assert_eq!(it.next().unwrap(), "chunk_aa.txt");
/// assert_eq!(it.next().unwrap(), "chunk_ab.txt");
/// assert_eq!(it.next().unwrap(), "chunk_ac.txt");
/// ```
///
/// For decimal numeric filenames, use `SuffixType::Decimal`:
///
/// ```rust,ignore
/// use crate::filenames::FilenameIterator;
/// use crate::filenames::SuffixType;
///
/// let prefix = "chunk_".to_string();
/// let suffix = ".txt".to_string();
/// let width = 2;
/// let suffix_type = SuffixType::Decimal;
/// let it = FilenameIterator::new(prefix, suffix, width, suffix_type);
///
/// assert_eq!(it.next().unwrap(), "chunk_00.txt");
/// assert_eq!(it.next().unwrap(), "chunk_01.txt");
/// assert_eq!(it.next().unwrap(), "chunk_02.txt");
/// ```
pub struct FilenameIterator<'a> {
    additional_suffix: &'a str,
    prefix: &'a str,
    number: Number,
    first_iteration: bool,
}

impl<'a> FilenameIterator<'a> {
    pub fn new(
        prefix: &'a str,
        additional_suffix: &'a str,
        suffix_length: usize,
        suffix_type: SuffixType,
    ) -> FilenameIterator<'a> {
        let radix = suffix_type.radix();
        let number = if suffix_length == 0 {
            Number::DynamicWidth(DynamicWidthNumber::new(radix))
        } else {
            Number::FixedWidth(FixedWidthNumber::new(radix, suffix_length))
        };
        FilenameIterator {
            prefix,
            additional_suffix,
            number,
            first_iteration: true,
        }
    }
}

impl<'a> Iterator for FilenameIterator<'a> {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        if self.first_iteration {
            self.first_iteration = false;
        } else {
            self.number.increment().ok()?;
        }
        // The first and third parts are just taken directly from the
        // struct parameters unchanged.
        Some(format!(
            "{}{}{}",
            self.prefix, self.number, self.additional_suffix
        ))
    }
}

#[cfg(test)]
mod tests {

    use crate::filenames::FilenameIterator;
    use crate::filenames::SuffixType;

    #[test]
    fn test_filename_iterator_alphabetic_fixed_width() {
        let mut it = FilenameIterator::new("chunk_", ".txt", 2, SuffixType::Alphabetic);
        assert_eq!(it.next().unwrap(), "chunk_aa.txt");
        assert_eq!(it.next().unwrap(), "chunk_ab.txt");
        assert_eq!(it.next().unwrap(), "chunk_ac.txt");

        let mut it = FilenameIterator::new("chunk_", ".txt", 2, SuffixType::Alphabetic);
        assert_eq!(it.nth(26 * 26 - 1).unwrap(), "chunk_zz.txt");
        assert_eq!(it.next(), None);
    }

    #[test]
    fn test_filename_iterator_numeric_fixed_width() {
        let mut it = FilenameIterator::new("chunk_", ".txt", 2, SuffixType::Decimal);
        assert_eq!(it.next().unwrap(), "chunk_00.txt");
        assert_eq!(it.next().unwrap(), "chunk_01.txt");
        assert_eq!(it.next().unwrap(), "chunk_02.txt");

        let mut it = FilenameIterator::new("chunk_", ".txt", 2, SuffixType::Decimal);
        assert_eq!(it.nth(10 * 10 - 1).unwrap(), "chunk_99.txt");
        assert_eq!(it.next(), None);
    }

    #[test]
    fn test_filename_iterator_alphabetic_dynamic_width() {
        let mut it = FilenameIterator::new("chunk_", ".txt", 0, SuffixType::Alphabetic);
        assert_eq!(it.next().unwrap(), "chunk_aa.txt");
        assert_eq!(it.next().unwrap(), "chunk_ab.txt");
        assert_eq!(it.next().unwrap(), "chunk_ac.txt");

        let mut it = FilenameIterator::new("chunk_", ".txt", 0, SuffixType::Alphabetic);
        assert_eq!(it.nth(26 * 25 - 1).unwrap(), "chunk_yz.txt");
        assert_eq!(it.next().unwrap(), "chunk_zaaa.txt");
        assert_eq!(it.next().unwrap(), "chunk_zaab.txt");
    }

    #[test]
    fn test_filename_iterator_numeric_dynamic_width() {
        let mut it = FilenameIterator::new("chunk_", ".txt", 0, SuffixType::Decimal);
        assert_eq!(it.next().unwrap(), "chunk_00.txt");
        assert_eq!(it.next().unwrap(), "chunk_01.txt");
        assert_eq!(it.next().unwrap(), "chunk_02.txt");

        let mut it = FilenameIterator::new("chunk_", ".txt", 0, SuffixType::Decimal);
        assert_eq!(it.nth(10 * 9 - 1).unwrap(), "chunk_89.txt");
        assert_eq!(it.next().unwrap(), "chunk_9000.txt");
        assert_eq!(it.next().unwrap(), "chunk_9001.txt");
    }
}
