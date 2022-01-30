// This file is part of the uutils coreutils package.
//
// (c) Rolf Morel <rolfmorel@gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) inval

use std::str::FromStr;

use crate::display::Quotable;

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Range {
    pub low: usize,
    pub high: usize,
}

impl FromStr for Range {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, &'static str> {
        use std::usize::MAX;

        let mut parts = s.splitn(2, '-');

        let field = "fields and positions are numbered from 1";
        let order = "high end of range less than low end";
        let inval = "failed to parse range";

        match (parts.next(), parts.next()) {
            (Some(nm), None) => {
                if let Ok(nm) = nm.parse::<usize>() {
                    if nm > 0 {
                        Ok(Self { low: nm, high: nm })
                    } else {
                        Err(field)
                    }
                } else {
                    Err(inval)
                }
            }
            (Some(n), Some(m)) if m.is_empty() => {
                if let Ok(low) = n.parse::<usize>() {
                    if low > 0 {
                        Ok(Self { low, high: MAX - 1 })
                    } else {
                        Err(field)
                    }
                } else {
                    Err(inval)
                }
            }
            (Some(n), Some(m)) if n.is_empty() => {
                if let Ok(high) = m.parse::<usize>() {
                    if high > 0 {
                        Ok(Self { low: 1, high })
                    } else {
                        Err(field)
                    }
                } else {
                    Err(inval)
                }
            }
            (Some(n), Some(m)) => match (n.parse::<usize>(), m.parse::<usize>()) {
                (Ok(low), Ok(high)) => {
                    if low > 0 && low <= high {
                        Ok(Self { low, high })
                    } else if low == 0 {
                        Err(field)
                    } else {
                        Err(order)
                    }
                }
                _ => Err(inval),
            },
            _ => unreachable!(),
        }
    }
}

impl Range {
    pub fn from_list(list: &str) -> Result<Vec<Self>, String> {
        use std::cmp::max;

        let mut ranges: Vec<Self> = vec![];

        for item in list.split(',') {
            let range_item = FromStr::from_str(item)
                .map_err(|e| format!("range {} was invalid: {}", item.quote(), e))?;
            ranges.push(range_item);
        }

        ranges.sort();

        // merge overlapping ranges
        for i in 0..ranges.len() {
            let j = i + 1;

            while j < ranges.len() && ranges[j].low <= ranges[i].high {
                let j_high = ranges.remove(j).high;
                ranges[i].high = max(ranges[i].high, j_high);
            }
        }

        Ok(ranges)
    }
}

pub fn complement(ranges: &[Range]) -> Vec<Range> {
    use std::usize;

    let mut complements = Vec::with_capacity(ranges.len() + 1);

    if !ranges.is_empty() && ranges[0].low > 1 {
        complements.push(Range {
            low: 1,
            high: ranges[0].low - 1,
        });
    }

    let mut ranges_iter = ranges.iter().peekable();
    loop {
        match (ranges_iter.next(), ranges_iter.peek()) {
            (Some(left), Some(right)) => {
                if left.high + 1 != right.low {
                    complements.push(Range {
                        low: left.high + 1,
                        high: right.low - 1,
                    });
                }
            }
            (Some(last), None) => {
                if last.high < usize::MAX - 1 {
                    complements.push(Range {
                        low: last.high + 1,
                        high: usize::MAX - 1,
                    });
                }
            }
            _ => break,
        }
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
