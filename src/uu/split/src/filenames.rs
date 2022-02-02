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
//!
//! let prefix = "chunk_".to_string();
//! let suffix = ".txt".to_string();
//! let width = 2;
//! let use_numeric_suffix = false;
//! let it = FilenameIterator::new(prefix, suffix, width, use_numeric_suffix);
//!
//! assert_eq!(it.next().unwrap(), "chunk_aa.txt");
//! assert_eq!(it.next().unwrap(), "chunk_ab.txt");
//! assert_eq!(it.next().unwrap(), "chunk_ac.txt");
//! ```
use crate::number::DynamicWidthNumber;
use crate::number::FixedWidthNumber;
use crate::number::Number;

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
/// Finally, if `use_numeric_suffix` is `true`, then numbers will be
/// used instead of lowercase ASCII alphabetic characters.
///
/// # Examples
///
/// Create filenames of the form `chunk_??.txt`, where the `?`
/// characters are lowercase ASCII alphabetic characters:
///
/// ```rust,ignore
/// use crate::filenames::FilenameIterator;
///
/// let prefix = "chunk_".to_string();
/// let suffix = ".txt".to_string();
/// let width = 2;
/// let use_numeric_suffix = false;
/// let it = FilenameIterator::new(prefix, suffix, width, use_numeric_suffix);
///
/// assert_eq!(it.next().unwrap(), "chunk_aa.txt");
/// assert_eq!(it.next().unwrap(), "chunk_ab.txt");
/// assert_eq!(it.next().unwrap(), "chunk_ac.txt");
/// ```
///
/// For numeric filenames, set `use_numeric_suffix` to `true`:
///
/// ```rust,ignore
/// use crate::filenames::FilenameIterator;
///
/// let prefix = "chunk_".to_string();
/// let suffix = ".txt".to_string();
/// let width = 2;
/// let use_numeric_suffix = true;
/// let it = FilenameIterator::new(prefix, suffix, width, use_numeric_suffix);
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
        use_numeric_suffix: bool,
    ) -> FilenameIterator<'a> {
        let radix = if use_numeric_suffix { 10 } else { 26 };
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

    #[test]
    fn test_filename_iterator_alphabetic_fixed_width() {
        let mut it = FilenameIterator::new("chunk_", ".txt", 2, false);
        assert_eq!(it.next().unwrap(), "chunk_aa.txt");
        assert_eq!(it.next().unwrap(), "chunk_ab.txt");
        assert_eq!(it.next().unwrap(), "chunk_ac.txt");

        let mut it = FilenameIterator::new("chunk_", ".txt", 2, false);
        assert_eq!(it.nth(26 * 26 - 1).unwrap(), "chunk_zz.txt");
        assert_eq!(it.next(), None);
    }

    #[test]
    fn test_filename_iterator_numeric_fixed_width() {
        let mut it = FilenameIterator::new("chunk_", ".txt", 2, true);
        assert_eq!(it.next().unwrap(), "chunk_00.txt");
        assert_eq!(it.next().unwrap(), "chunk_01.txt");
        assert_eq!(it.next().unwrap(), "chunk_02.txt");

        let mut it = FilenameIterator::new("chunk_", ".txt", 2, true);
        assert_eq!(it.nth(10 * 10 - 1).unwrap(), "chunk_99.txt");
        assert_eq!(it.next(), None);
    }

    #[test]
    fn test_filename_iterator_alphabetic_dynamic_width() {
        let mut it = FilenameIterator::new("chunk_", ".txt", 0, false);
        assert_eq!(it.next().unwrap(), "chunk_aa.txt");
        assert_eq!(it.next().unwrap(), "chunk_ab.txt");
        assert_eq!(it.next().unwrap(), "chunk_ac.txt");

        let mut it = FilenameIterator::new("chunk_", ".txt", 0, false);
        assert_eq!(it.nth(26 * 25 - 1).unwrap(), "chunk_yz.txt");
        assert_eq!(it.next().unwrap(), "chunk_zaaa.txt");
        assert_eq!(it.next().unwrap(), "chunk_zaab.txt");
    }

    #[test]
    fn test_filename_iterator_numeric_dynamic_width() {
        let mut it = FilenameIterator::new("chunk_", ".txt", 0, true);
        assert_eq!(it.next().unwrap(), "chunk_00.txt");
        assert_eq!(it.next().unwrap(), "chunk_01.txt");
        assert_eq!(it.next().unwrap(), "chunk_02.txt");

        let mut it = FilenameIterator::new("chunk_", ".txt", 0, true);
        assert_eq!(it.nth(10 * 9 - 1).unwrap(), "chunk_89.txt");
        assert_eq!(it.next().unwrap(), "chunk_9000.txt");
        assert_eq!(it.next().unwrap(), "chunk_9001.txt");
    }
}
