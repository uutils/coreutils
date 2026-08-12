// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) inval

//! A module for handling ranges of values.

use std::cmp::max;
use std::ops::Range as ByteRange;
use std::str::FromStr;

use crate::display::Quotable;

/// A range of values
#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Range {
    /// The lower bound of the range
    pub low: usize,

    /// The upper bound of the range
    pub high: usize,
}

impl FromStr for Range {
    type Err = &'static str;

    /// Parse a string of the form `a-b` into a `Range`
    ///
    /// ```
    /// use std::str::FromStr;
    /// use uucore::ranges::Range;
    /// assert_eq!(Range::from_str("5"), Ok(Range { low: 5, high: 5 }));
    /// assert_eq!(Range::from_str("4-"), Ok(Range { low: 4, high: usize::MAX - 1 }));
    /// assert_eq!(Range::from_str("-4"), Ok(Range { low: 1, high: 4 }));
    /// assert_eq!(Range::from_str("2-4"), Ok(Range { low: 2, high: 4 }));
    /// assert!(Range::from_str("0-4").is_err());
    /// assert!(Range::from_str("4-2").is_err());
    /// assert!(Range::from_str("-").is_err());
    /// assert!(Range::from_str("a").is_err());
    /// assert!(Range::from_str("a-b").is_err());
    /// ```
    fn from_str(s: &str) -> Result<Self, &'static str> {
        Self::parse(s).map_err(|invalid| invalid.reason)
    }
}

/// What is wrong with a range, so that a caller can label a caret.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RangeErrorKind {
    /// A bound that is not a number at all.
    NotANumber,
    /// A bound of zero, where counting starts at one.
    ZeroBound,
    /// A bound too large to be used.
    TooLarge,
    /// A bare `-`, with a bound on neither side.
    NoEndpoint,
    /// A range that ends before it starts.
    Inverted,
}

/// A list of ranges that does not parse, and the part of it that is at fault.
///
/// The message is built here so that it reads exactly as it always has; `span`
/// and `kind` are what a caret needs on top of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RangeError {
    pub message: String,
    /// Byte range inside the whole list, not inside the one item at fault.
    pub span: ByteRange<usize>,
    pub kind: RangeErrorKind,
}

impl std::fmt::Display for RangeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for RangeError {}

impl RangeError {
    /// Render this error against `args`, with a caret under the one range in
    /// `list` that is at fault.
    ///
    /// A list of ranges reads the same wherever it is taken, so what a caret
    /// says about a bad one is written here rather than in each utility. Only
    /// the two texts that genuinely differ between them are passed in: what a
    /// range counts, and how the option spells a list.
    ///
    /// # Arguments
    ///
    /// * `args` - The whole argument list, program name included — as
    ///   [`crate::diagnostics::capture`] returns it.
    /// * `list` - The list of ranges as typed, the value of the option below.
    /// * `short` - The short name of that option, if it has one.
    /// * `long` - Its long name, if it has one.
    /// * `zero_bound` - Already localized: what a zero bound got wrong, which
    ///   depends on what the range counts — bytes, fields, characters.
    /// * `help` - Already localized: how a list is written for this option.
    ///
    /// # Returns
    ///
    /// `false` when no argument carries `list` as that option's value, in which
    /// case the caller should fall back to the plain one-line message.
    pub fn render_option_value(
        &self,
        args: &[std::ffi::OsString],
        list: &str,
        short: Option<char>,
        long: Option<&str>,
        zero_bound: &str,
        help: &str,
    ) -> bool {
        // Labelled only where a label would add to the message, per the
        // convention in `crate::diagnostics`.
        let label = match self.kind {
            RangeErrorKind::NotANumber | RangeErrorKind::NoEndpoint => None,
            RangeErrorKind::ZeroBound => Some(std::borrow::Cow::Borrowed(zero_bound)),
            RangeErrorKind::TooLarge => Some(std::borrow::Cow::Owned(crate::translate!(
                "range-diag-label-too-large"
            ))),
            RangeErrorKind::Inverted => Some(std::borrow::Cow::Owned(crate::translate!(
                "range-diag-label-inverted"
            ))),
        };

        crate::diagnostics::Snapshot::with_program(args).render_option_value(
            list,
            short,
            long,
            self.span.clone(),
            &self.message,
            label.as_deref(),
            Some(help),
        )
    }
}

/// One range that does not parse, located inside the item it was read from.
struct Invalid {
    reason: &'static str,
    span: ByteRange<usize>,
    kind: RangeErrorKind,
}

impl Range {
    /// Parse one range, reporting where inside `s` the trouble is.
    fn parse(s: &str) -> Result<Self, Invalid> {
        // Each bound is parsed where it sits, so that a caret can point at the
        // one that is wrong rather than at the pair.
        let bound = |part: &str, offset: usize| -> Result<usize, Invalid> {
            let at = |reason, kind| Invalid {
                reason,
                span: offset..offset + part.len(),
                kind,
            };
            match part.parse::<usize>() {
                Ok(0) => Err(at(
                    "fields and positions are numbered from 1",
                    RangeErrorKind::ZeroBound,
                )),
                // GNU fails when we are at the limit. Match their behavior
                Ok(n) if n == usize::MAX => Err(at(
                    "byte/character offset is too large",
                    RangeErrorKind::TooLarge,
                )),
                Ok(n) => Ok(n),
                Err(_) => Err(at("failed to parse range", RangeErrorKind::NotANumber)),
            }
        };
        // Everything after the dash starts one byte past it.
        let after_dash = |low: &str| low.len() + 1;

        Ok(match s.split_once('-') {
            None => {
                let n = bound(s, 0)?;
                Self { low: n, high: n }
            }
            Some(("", "")) => {
                return Err(Invalid {
                    reason: "invalid range with no endpoint",
                    span: 0..s.len(),
                    kind: RangeErrorKind::NoEndpoint,
                });
            }
            Some((low, "")) => Self {
                low: bound(low, 0)?,
                high: usize::MAX - 1,
            },
            Some(("", high)) => Self {
                low: 1,
                high: bound(high, after_dash(""))?,
            },
            Some((low, high)) => {
                let (low_value, high_value) = (bound(low, 0)?, bound(high, after_dash(low))?);
                if low_value <= high_value {
                    Self {
                        low: low_value,
                        high: high_value,
                    }
                } else {
                    return Err(Invalid {
                        reason: "high end of range less than low end",
                        // Neither bound is wrong on its own; it is the pair.
                        span: 0..s.len(),
                        kind: RangeErrorKind::Inverted,
                    });
                }
            }
        })
    }
}

impl Range {
    /// Parse a list of ranges separated by commas and/or spaces
    ///
    /// # Returns
    ///
    /// On failure, the message as it has always read, and where in `list` the
    /// item at fault sits, so that a caller may point a caret at it.
    pub fn from_list(list: &str) -> Result<Vec<Self>, RangeError> {
        let mut ranges = Vec::new();

        // Where the item being read starts inside the whole list. The
        // separators are one byte each, so stepping over them is enough to
        // keep count.
        let mut start = 0;
        for item in list.split(&[',', ' ']) {
            let range_item = Self::parse(item).map_err(|invalid| RangeError {
                message: format!("range {} was invalid: {}", item.quote(), invalid.reason),
                span: start + invalid.span.start..start + invalid.span.end,
                kind: invalid.kind,
            })?;
            ranges.push(range_item);
            start += item.len() + 1;
        }

        Ok(Self::merge(ranges))
    }

    /// Merge any overlapping ranges. Adjacent ranges are *NOT* merged.
    ///
    /// Is guaranteed to return only disjoint ranges in a sorted order.
    pub fn merge(mut ranges: Vec<Self>) -> Vec<Self> {
        ranges.sort();

        // merge overlapping ranges
        for i in 0..ranges.len() {
            let j = i + 1;

            while j < ranges.len() && ranges[j].low <= ranges[i].high {
                let j_high = ranges.remove(j).high;
                ranges[i].high = max(ranges[i].high, j_high);
            }
        }
        ranges
    }
}

/// Calculate the complement of the given ranges.
pub fn complement(ranges: &[Range]) -> Vec<Range> {
    let mut prev_high = 0;
    let mut complements = Vec::with_capacity(ranges.len() + 1);

    for range in ranges {
        if range.low > prev_high + 1 {
            complements.push(Range {
                low: prev_high + 1,
                high: range.low - 1,
            });
        }
        prev_high = range.high;
    }

    if prev_high < usize::MAX - 1 {
        complements.push(Range {
            low: prev_high + 1,
            high: usize::MAX - 1,
        });
    }

    complements
}

/// Test if at least one of the given Ranges contain the supplied value.
///
/// Examples:
///
/// ```
/// let ranges = uucore::ranges::Range::from_list("11,2,6-8").unwrap();
///
/// assert!(!uucore::ranges::contain(&ranges, 0));
/// assert!(!uucore::ranges::contain(&ranges, 1));
/// assert!(!uucore::ranges::contain(&ranges, 5));
/// assert!(!uucore::ranges::contain(&ranges, 10));
///
/// assert!(uucore::ranges::contain(&ranges, 2));
/// assert!(uucore::ranges::contain(&ranges, 6));
/// assert!(uucore::ranges::contain(&ranges, 7));
/// assert!(uucore::ranges::contain(&ranges, 8));
/// assert!(uucore::ranges::contain(&ranges, 11));
/// ```
pub fn contain(ranges: &[Range], n: usize) -> bool {
    for range in ranges {
        if n >= range.low && n <= range.high {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod test {
    use super::{Range, complement};
    use std::str::FromStr;

    fn m(a: Vec<Range>, b: &[Range]) {
        assert_eq!(Range::merge(a), b);
    }

    fn r(low: usize, high: usize) -> Range {
        Range { low, high }
    }

    #[test]
    fn merging() {
        // Single element
        m(vec![r(1, 2)], &[r(1, 2)]);

        // Disjoint in wrong order
        m(vec![r(4, 5), r(1, 2)], &[r(1, 2), r(4, 5)]);

        // Two elements must be merged
        m(vec![r(1, 3), r(2, 4), r(6, 7)], &[r(1, 4), r(6, 7)]);

        // Two merges and a duplicate
        m(
            vec![r(1, 3), r(6, 7), r(2, 4), r(6, 7)],
            &[r(1, 4), r(6, 7)],
        );

        // One giant
        m(
            vec![
                r(110, 120),
                r(10, 20),
                r(100, 200),
                r(130, 140),
                r(150, 160),
            ],
            &[r(10, 20), r(100, 200)],
        );

        // Last one joins the previous two
        m(vec![r(10, 20), r(30, 40), r(20, 30)], &[r(10, 40)]);

        m(
            vec![r(10, 20), r(30, 40), r(50, 60), r(20, 30)],
            &[r(10, 40), r(50, 60)],
        );

        // Don't merge adjacent ranges
        m(vec![r(1, 3), r(4, 6)], &[r(1, 3), r(4, 6)]);
    }

    #[test]
    fn complementing() {
        // Simple
        assert_eq!(complement(&[r(3, 4)]), vec![r(1, 2), r(5, usize::MAX - 1)]);

        // With start
        assert_eq!(
            complement(&[r(1, 3), r(6, 10)]),
            vec![r(4, 5), r(11, usize::MAX - 1)]
        );

        // With end
        assert_eq!(
            complement(&[r(2, 4), r(6, usize::MAX - 1)]),
            vec![r(1, 1), r(5, 5)]
        );

        // With start and end
        assert_eq!(complement(&[r(1, 4), r(6, usize::MAX - 1)]), vec![r(5, 5)]);
    }

    #[test]
    fn test_from_str() {
        assert_eq!(Range::from_str("5"), Ok(Range { low: 5, high: 5 }));
        assert_eq!(Range::from_str("3-5"), Ok(Range { low: 3, high: 5 }));
        assert_eq!(
            Range::from_str("5-3"),
            Err("high end of range less than low end")
        );
        assert_eq!(Range::from_str("-"), Err("invalid range with no endpoint"));
        assert_eq!(
            Range::from_str("3-"),
            Ok(Range {
                low: 3,
                high: usize::MAX - 1
            })
        );
        assert_eq!(Range::from_str("-5"), Ok(Range { low: 1, high: 5 }));
        assert_eq!(
            Range::from_str("0"),
            Err("fields and positions are numbered from 1")
        );

        let max_value = format!("{}", usize::MAX);
        assert_eq!(
            Range::from_str(&max_value),
            Err("byte/character offset is too large")
        );
    }
}
