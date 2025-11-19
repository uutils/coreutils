// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore zaaa zaab stype
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
//! let suffix = Suffix {
//!     stype: SuffixType::Alphabetic,
//!     length: 2,
//!     start: 0,
//!     auto_widening: true,
//!     additional: ".txt".to_string(),
//! };
//! let it = FilenameIterator::new(prefix, suffix);
//!
//! assert_eq!(it.next().unwrap(), "chunk_aa.txt");
//! assert_eq!(it.next().unwrap(), "chunk_ab.txt");
//! assert_eq!(it.next().unwrap(), "chunk_ac.txt");
//! ```

use crate::number::DynamicWidthNumber;
use crate::number::FixedWidthNumber;
use crate::number::Number;
use crate::strategy::Strategy;
use crate::{
    OPT_ADDITIONAL_SUFFIX, OPT_HEX_SUFFIXES, OPT_HEX_SUFFIXES_SHORT, OPT_NUMERIC_SUFFIXES,
    OPT_NUMERIC_SUFFIXES_SHORT, OPT_SUFFIX_LENGTH,
};
use clap::ArgMatches;
use std::ffi::{OsStr, OsString};
use std::path::is_separator;
use thiserror::Error;
use uucore::display::Quotable;
use uucore::error::{UResult, USimpleError};
use uucore::translate;

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

/// Filename suffix parameters
#[derive(Clone)]
pub struct Suffix {
    stype: SuffixType,
    length: usize,
    start: usize,
    auto_widening: bool,
    additional: OsString,
}

/// An error when parsing suffix parameters from command-line arguments.
#[derive(Debug, Error)]
pub enum SuffixError {
    /// Invalid suffix length parameter.
    #[error("{}", translate!("split-error-suffix-not-parsable", "value" => .0.quote()))]
    NotParsable(String),

    /// Suffix contains a directory separator, which is not allowed.
    #[error("{}", translate!("split-error-suffix-contains-separator", "value" => .0.quote()))]
    ContainsSeparator(OsString),

    /// Suffix is not large enough to split into specified chunks
    #[error("{}", translate!("split-error-suffix-too-small", "length" => .0))]
    TooSmall(usize),
}

impl Suffix {
    /// Parse the suffix type, start, length and additional suffix from the command-line arguments
    /// as well process suffix length auto-widening and auto-width scenarios
    ///
    /// Suffix auto-widening: Determine if the output file names suffix is allowed to dynamically auto-widen,
    /// i.e. change (increase) suffix length dynamically as more files need to be written into.
    /// Suffix length auto-widening rules are (in the order they are applied):
    /// - ON by default
    /// - OFF when suffix start N is specified via long option with a value
    ///   `--numeric-suffixes=N` or `--hex-suffixes=N`
    /// - OFF when suffix length N is specified, except for N=0 (see edge cases below)
    ///   `-a N` or `--suffix-length=N`
    /// - OFF if suffix length is auto pre-calculated (auto-width)
    ///
    /// Suffix auto-width: Determine if the the output file names suffix length should be automatically pre-calculated
    /// based on number of files that need to written into, having number of files known upfront
    /// Suffix length auto pre-calculation rules:
    /// - Pre-calculate new suffix length when `-n`/`--number` option (N, K/N, l/N, l/K/N, r/N, r/K/N)
    ///   is used, where N is number of chunks = number of files to write into
    ///   and suffix start < N number of files
    ///   as in `split --numeric-suffixes=1 --number=r/100 file`
    /// - Do NOT pre-calculate new suffix length otherwise, i.e. when
    ///   suffix start >= N number of files
    ///   as in `split --numeric-suffixes=100 --number=r/100 file`
    ///   OR when suffix length N is specified, except for N=0 (see edge cases below)
    ///   `-a N` or `--suffix-length=N`
    ///
    /// Edge case:
    /// - If suffix length is specified as 0 in a command line,
    ///   first apply auto-width calculations and if still 0
    ///   set it to default value.
    ///   Do NOT change auto-widening value
    ///
    pub fn from(matches: &ArgMatches, strategy: &Strategy) -> Result<Self, SuffixError> {
        let stype: SuffixType;

        // Defaults
        let mut start = 0;
        let mut auto_widening = true;
        let default_length: usize = 2;

        // Check if the user is specifying one or more than one suffix
        // Any combination of suffixes is allowed
        // Since all suffixes are setup with 'overrides_with_all()' against themselves and each other,
        // last one wins, all others are ignored
        match (
            matches.contains_id(OPT_NUMERIC_SUFFIXES),
            matches.contains_id(OPT_HEX_SUFFIXES),
            matches.get_flag(OPT_NUMERIC_SUFFIXES_SHORT),
            matches.get_flag(OPT_HEX_SUFFIXES_SHORT),
        ) {
            (true, _, _, _) => {
                stype = SuffixType::Decimal;
                // if option was specified, but without value - this will return None as there is no default value
                if let Some(opt) = matches.get_one::<String>(OPT_NUMERIC_SUFFIXES) {
                    start = opt
                        .parse::<usize>()
                        .map_err(|_| SuffixError::NotParsable(opt.to_owned()))?;
                    auto_widening = false;
                }
            }
            (_, true, _, _) => {
                stype = SuffixType::Hexadecimal;
                // if option was specified, but without value - this will return None as there is no default value
                if let Some(opt) = matches.get_one::<String>(OPT_HEX_SUFFIXES) {
                    start = usize::from_str_radix(opt, 16)
                        .map_err(|_| SuffixError::NotParsable(opt.to_owned()))?;
                    auto_widening = false;
                }
            }
            (_, _, true, _) => stype = SuffixType::Decimal, // short numeric suffix '-d'
            (_, _, _, true) => stype = SuffixType::Hexadecimal, // short hex suffix '-x'
            _ => stype = SuffixType::Alphabetic, // no numeric/hex suffix, using default alphabetic
        }

        // Get suffix length and a flag to indicate if it was specified with command line option
        let (mut length, is_length_cmd_opt) =
            if let Some(v) = matches.get_one::<String>(OPT_SUFFIX_LENGTH) {
                // suffix length was specified in command line
                (
                    v.parse::<usize>()
                        .map_err(|_| SuffixError::NotParsable(v.to_owned()))?,
                    true,
                )
            } else {
                // no suffix length option was specified in command line
                // set to default value
                (default_length, false)
            };

        // Disable dynamic auto-widening if suffix length was specified in command line with value > 0
        if is_length_cmd_opt && length > 0 {
            auto_widening = false;
        }

        // Auto pre-calculate new suffix length (auto-width) if necessary
        if let Strategy::Number(number_type) = strategy {
            let chunks = number_type.num_chunks();
            let required_length = ((start as u64 + chunks) as f64)
                .log(stype.radix() as f64)
                .ceil() as usize;

            if (start as u64) < chunks && !(is_length_cmd_opt && length > 0) {
                // with auto-width ON the auto-widening is OFF
                auto_widening = false;

                // do not reduce suffix length with auto-width
                if length < required_length {
                    length = required_length;
                }
            }

            if length < required_length {
                return Err(SuffixError::TooSmall(required_length));
            }
        }

        // Check edge case when suffix length == 0 was specified in command line
        // Set it to default value
        if is_length_cmd_opt && length == 0 {
            length = default_length;
        }

        let additional = matches
            .get_one::<OsString>(OPT_ADDITIONAL_SUFFIX)
            .unwrap()
            .clone();
        if additional.to_string_lossy().chars().any(is_separator) {
            return Err(SuffixError::ContainsSeparator(additional));
        }

        let result = Self {
            stype,
            length,
            start,
            auto_widening,
            additional,
        };

        Ok(result)
    }
}

/// Compute filenames from a given index.
///
/// This iterator yields filenames for use with ``split``.
///
/// The `prefix` is prepended to each filename and the
/// `suffix.additional` is appended to each filename.
///
/// If `suffix.auto_widening` is true, then the variable portion of the filename
/// that identifies the current chunk will have a dynamically
/// increasing width. If `suffix.auto_widening` is false, then
/// the variable portion of the filename will always be exactly `suffix.length`
/// width in characters. In that case, after the iterator yields each
/// string of that width, the iterator is exhausted.
///
/// Finally, `suffix.stype` controls which type of suffix to produce,
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
/// let suffix = Suffix {
///     stype: SuffixType::Alphabetic,
///     length: 2,
///     start: 0,
///     auto_widening: true,
///     additional: ".txt".to_string(),
/// };
/// let it = FilenameIterator::new(prefix, suffix);
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
/// let suffix = Suffix {
///     stype: SuffixType::Decimal,
///     length: 2,
///     start: 0,
///     auto_widening: true,
///     additional: ".txt".to_string(),
/// };
/// let it = FilenameIterator::new(prefix, suffix);
///
/// assert_eq!(it.next().unwrap(), "chunk_00.txt");
/// assert_eq!(it.next().unwrap(), "chunk_01.txt");
/// assert_eq!(it.next().unwrap(), "chunk_02.txt");
/// ```
pub struct FilenameIterator<'a> {
    prefix: &'a OsStr,
    additional_suffix: &'a OsStr,
    number: Number,
    first_iteration: bool,
}

impl<'a> FilenameIterator<'a> {
    pub fn new(prefix: &'a OsStr, suffix: &'a Suffix) -> UResult<Self> {
        let radix = suffix.stype.radix();
        let number = if suffix.auto_widening {
            Number::DynamicWidth(DynamicWidthNumber::new(radix, suffix.start))
        } else {
            Number::FixedWidth(
                FixedWidthNumber::new(radix, suffix.length, suffix.start).map_err(|_| {
                    USimpleError::new(
                        1,
                        translate!("split-error-numerical-suffix-start-too-large"),
                    )
                })?,
            )
        };
        let additional_suffix = &suffix.additional;

        Ok(FilenameIterator {
            prefix,
            additional_suffix,
            number,
            first_iteration: true,
        })
    }
}

impl Iterator for FilenameIterator<'_> {
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
            self.prefix.to_string_lossy(),
            self.number,
            self.additional_suffix.to_string_lossy()
        ))
    }
}

#[cfg(test)]
mod tests {

    use crate::filenames::FilenameIterator;
    use crate::filenames::Suffix;
    use crate::filenames::SuffixType;

    #[test]
    fn test_filename_iterator_alphabetic_fixed_width() {
        let suffix = Suffix {
            stype: SuffixType::Alphabetic,
            length: 2,
            start: 0,
            auto_widening: false,
            additional: ".txt".into(),
        };
        let mut it = FilenameIterator::new(std::ffi::OsStr::new("chunk_"), &suffix).unwrap();
        assert_eq!(it.next().unwrap(), "chunk_aa.txt");
        assert_eq!(it.next().unwrap(), "chunk_ab.txt");
        assert_eq!(it.next().unwrap(), "chunk_ac.txt");

        let mut it = FilenameIterator::new(std::ffi::OsStr::new("chunk_"), &suffix).unwrap();
        assert_eq!(it.nth(26 * 26 - 1).unwrap(), "chunk_zz.txt");
        assert_eq!(it.next(), None);
    }

    #[test]
    fn test_filename_iterator_numeric_fixed_width() {
        let suffix = Suffix {
            stype: SuffixType::Decimal,
            length: 2,
            start: 0,
            auto_widening: false,
            additional: ".txt".into(),
        };
        let mut it = FilenameIterator::new(std::ffi::OsStr::new("chunk_"), &suffix).unwrap();
        assert_eq!(it.next().unwrap(), "chunk_00.txt");
        assert_eq!(it.next().unwrap(), "chunk_01.txt");
        assert_eq!(it.next().unwrap(), "chunk_02.txt");

        let mut it = FilenameIterator::new(std::ffi::OsStr::new("chunk_"), &suffix).unwrap();
        assert_eq!(it.nth(10 * 10 - 1).unwrap(), "chunk_99.txt");
        assert_eq!(it.next(), None);
    }

    #[test]
    fn test_filename_iterator_alphabetic_dynamic_width() {
        let suffix = Suffix {
            stype: SuffixType::Alphabetic,
            length: 2,
            start: 0,
            auto_widening: true,
            additional: ".txt".into(),
        };
        let mut it = FilenameIterator::new(std::ffi::OsStr::new("chunk_"), &suffix).unwrap();
        assert_eq!(it.next().unwrap(), "chunk_aa.txt");
        assert_eq!(it.next().unwrap(), "chunk_ab.txt");
        assert_eq!(it.next().unwrap(), "chunk_ac.txt");

        let mut it = FilenameIterator::new(std::ffi::OsStr::new("chunk_"), &suffix).unwrap();
        assert_eq!(it.nth(26 * 25 - 1).unwrap(), "chunk_yz.txt");
        assert_eq!(it.next().unwrap(), "chunk_zaaa.txt");
        assert_eq!(it.next().unwrap(), "chunk_zaab.txt");
    }

    #[test]
    fn test_filename_iterator_numeric_dynamic_width() {
        let suffix = Suffix {
            stype: SuffixType::Decimal,
            length: 2,
            start: 0,
            auto_widening: true,
            additional: ".txt".into(),
        };
        let mut it = FilenameIterator::new(std::ffi::OsStr::new("chunk_"), &suffix).unwrap();
        assert_eq!(it.next().unwrap(), "chunk_00.txt");
        assert_eq!(it.next().unwrap(), "chunk_01.txt");
        assert_eq!(it.next().unwrap(), "chunk_02.txt");

        let mut it = FilenameIterator::new(std::ffi::OsStr::new("chunk_"), &suffix).unwrap();
        assert_eq!(it.nth(10 * 9 - 1).unwrap(), "chunk_89.txt");
        assert_eq!(it.next().unwrap(), "chunk_9000.txt");
        assert_eq!(it.next().unwrap(), "chunk_9001.txt");
    }

    #[test]
    fn test_filename_iterator_numeric_decimal() {
        let suffix = Suffix {
            stype: SuffixType::Decimal,
            length: 2,
            start: 5,
            auto_widening: true,
            additional: ".txt".into(),
        };
        let mut it = FilenameIterator::new(std::ffi::OsStr::new("chunk_"), &suffix).unwrap();
        assert_eq!(it.next().unwrap(), "chunk_05.txt");
        assert_eq!(it.next().unwrap(), "chunk_06.txt");
        assert_eq!(it.next().unwrap(), "chunk_07.txt");
    }

    #[test]
    fn test_filename_iterator_numeric_hex() {
        let suffix = Suffix {
            stype: SuffixType::Hexadecimal,
            length: 2,
            start: 9,
            auto_widening: true,
            additional: ".txt".into(),
        };
        let mut it = FilenameIterator::new(std::ffi::OsStr::new("chunk_"), &suffix).unwrap();
        assert_eq!(it.next().unwrap(), "chunk_09.txt");
        assert_eq!(it.next().unwrap(), "chunk_0a.txt");
        assert_eq!(it.next().unwrap(), "chunk_0b.txt");
    }

    #[test]
    fn test_filename_iterator_numeric_err() {
        let suffix = Suffix {
            stype: SuffixType::Decimal,
            length: 3,
            start: 999,
            auto_widening: false,
            additional: ".txt".into(),
        };
        let mut it = FilenameIterator::new(std::ffi::OsStr::new("chunk_"), &suffix).unwrap();
        assert_eq!(it.next().unwrap(), "chunk_999.txt");
        assert!(it.next().is_none());

        let suffix = Suffix {
            stype: SuffixType::Decimal,
            length: 3,
            start: 1000,
            auto_widening: false,
            additional: ".txt".into(),
        };
        let it = FilenameIterator::new(std::ffi::OsStr::new("chunk_"), &suffix);
        assert!(it.is_err());

        let suffix = Suffix {
            stype: SuffixType::Hexadecimal,
            length: 3,
            start: 0xfff,
            auto_widening: false,
            additional: ".txt".into(),
        };
        let mut it = FilenameIterator::new(std::ffi::OsStr::new("chunk_"), &suffix).unwrap();
        assert_eq!(it.next().unwrap(), "chunk_fff.txt");
        assert!(it.next().is_none());

        let suffix = Suffix {
            stype: SuffixType::Hexadecimal,
            length: 3,
            start: 0x1000,
            auto_widening: false,
            additional: ".txt".into(),
        };
        let it = FilenameIterator::new(std::ffi::OsStr::new("chunk_"), &suffix);
        assert!(it.is_err());
    }
}
