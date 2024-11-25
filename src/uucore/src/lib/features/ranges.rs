// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) inval

//! A module for handling ranges of values.

use std::cmp::max;
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
        fn parse(s: &str) -> Result<usize, &'static str> {
            match s.parse::<usize>() {
                Ok(0) => Err("fields and positions are numbered from 1"),
                // GNU fails when we are at the limit. Match their behavior
                Ok(n) if n == usize::MAX => Err("byte/character offset is too large"),
                Ok(n) => Ok(n),
                Err(_) => Err("failed to parse range"),
            }
        }

        Ok(match s.split_once('-') {
            None => {
                let n = parse(s)?;
                Self { low: n, high: n }
            }
            Some(("", "")) => return Err("invalid range with no endpoint"),
            Some((low, "")) => Self {
                low: parse(low)?,
                high: usize::MAX - 1,
            },
            Some(("", high)) => Self {
                low: 1,
                high: parse(high)?,
            },
            Some((low, high)) => {
                let (low, high) = (parse(low)?, parse(high)?);
                if low <= high {
                    Self { low, high }
                } else {
                    return Err("high end of range less than low end");
                }
            }
        })
    }
}

impl Range {
    /// Parse a list of ranges separated by commas and/or spaces
    pub fn from_list(list: &str) -> Result<Vec<Self>, String> {
        let mut ranges = Vec::new();

        for item in list.split(&[',', ' ']) {
            let range_item = FromStr::from_str(item)
                .map_err(|e| format!("range {} was invalid: {}", item.quote(), e))?;
            ranges.push(range_item);
        }

        Ok(Self::merge(ranges))
    }

    /// Merge any overlapping ranges
    ///
    /// Is guaranteed to return only disjoint ranges in a sorted order.
    fn merge(mut ranges: Vec<Self>) -> Vec<Self> {
        ranges.sort();

        // merge overlapping ranges
        for i in 0..ranges.len() {
            let j = i + 1;

            // The +1 is a small optimization, because we can merge adjacent Ranges.
            // For example (1,3) and (4,6), because in the integers, there are no
            // possible values between 3 and 4, this is equivalent to (1,6).
            while j < ranges.len() && ranges[j].low <= ranges[i].high + 1 {
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
    use super::{complement, Range};
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

        // Merge adjacent ranges
        m(vec![r(1, 3), r(4, 6)], &[r(1, 6)]);
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
