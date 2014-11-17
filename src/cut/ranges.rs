/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Rolf Morel <rolfmorel@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

use std;

#[deriving(PartialEq,Eq,PartialOrd,Ord,Show)]
pub struct Range {
    pub low: uint,
    pub high: uint,
}

impl std::str::FromStr for Range {
    fn from_str(s: &str) -> Option<Range> {
        use std::uint::MAX;

        let mut parts = s.splitn(1, '-');

        match (parts.next(), parts.next()) {
            (Some(nm), None) => {
                from_str::<uint>(nm).and_then(|nm| if nm > 0 { Some(nm) } else { None })
                                    .map(|nm| Range { low: nm, high: nm })
            }
            (Some(n), Some(m)) if m.len() == 0 => {
                from_str::<uint>(n).and_then(|low| if low > 0 { Some(low) } else { None })
                                   .map(|low| Range { low: low, high: MAX })
            }
            (Some(n), Some(m)) if n.len() == 0 => {
                from_str::<uint>(m).and_then(|high| if high >= 1 { Some(high) } else { None })
                                   .map(|high| Range { low: 1, high: high })
            }
            (Some(n), Some(m)) => {
                match (from_str::<uint>(n), from_str::<uint>(m)) {
                    (Some(low), Some(high)) if low > 0 && low <= high => {
                        Some(Range { low: low, high: high })
                    }
                    _ => None
                }
            }
            _ => unreachable!()
        }
    }
}

impl Range {
    pub fn from_list(list: &str) -> Result<Vec<Range>, String> {
        use std::cmp::max;

        let mut ranges = vec!();

        for item in list.split(',') {
            match from_str::<Range>(item) {
                Some(range_item) => ranges.push(range_item),
                None => return Err(format!("range '{}' was invalid", item))
            }
        }

        ranges.sort();

        // merge overlapping ranges
        for i in range(0, ranges.len()) {
            let j = i + 1;

            while j < ranges.len() && ranges[j].low <= ranges[i].high {
                let j_high = ranges.remove(j).unwrap().high;
                ranges[i].high = max(ranges[i].high, j_high);
            }
        }

        Ok(ranges)
    }
}

pub fn complement(ranges: &Vec<Range>) -> Vec<Range> {
    use std::uint;

    let mut complements = Vec::with_capacity(ranges.len() + 1);

    if ranges.len() > 0 && ranges[0].low > 1 {
        complements.push(Range { low: 1, high: ranges[0].low - 1 });
    }

    let mut ranges_iter = ranges.iter().peekable();
    loop {
        match (ranges_iter.next(), ranges_iter.peek()) {
            (Some(left), Some(right)) => {
                if left.high + 1 != right.low {
                    complements.push(Range {
                                         low: left.high + 1,
                                         high: right.low - 1
                                     });
                }
            }
            (Some(last), None) => {
                if last.high < uint::MAX {
                    complements.push(Range {
                                        low: last.high + 1,
                                        high: uint::MAX
                                     });
                }
            }
            _ => break
        }
    }

    complements
}
