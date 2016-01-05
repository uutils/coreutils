/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Rolf Morel <rolfmorel@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

use std::str::FromStr;

#[derive(PartialEq,Eq,PartialOrd,Ord,Debug)]
pub struct Range {
    pub low: usize,
    pub high: usize,
}

impl FromStr for Range {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Range, &'static str> {
        use std::usize::MAX;

        let mut parts = s.splitn(2, '-');

        let field = "fields and positions are numbered from 1";
        let order = "high end of range less than low end";
        let inval = "failed to parse range";

        match (parts.next(), parts.next()) {
            (Some(nm), None) => {
                if let Ok(nm) = nm.parse::<usize>() {
                    if nm > 0 { Ok(Range{ low: nm, high: nm}) } else { Err(field) }
                } else {
                    Err(inval)
                }
            }
            (Some(n), Some(m)) if m.len() == 0 => {
                if let Ok(low) = n.parse::<usize>() {
                    if low > 0 { Ok(Range{ low: low, high: MAX - 1}) } else { Err(field) }
                } else {
                    Err(inval)
                }
            }
            (Some(n), Some(m)) if n.len() == 0 => {
                if let Ok(high) = m.parse::<usize>() {
                    if high > 0 { Ok(Range{ low: 1, high: high}) } else { Err(field) }
                } else {
                    Err(inval)
                }
            }
            (Some(n), Some(m)) => {
                match (n.parse::<usize>(), m.parse::<usize>()) {
                    (Ok(low), Ok(high)) => {
                        if low > 0 && low <= high {
                            Ok(Range { low: low, high: high })
                        } else if low == 0 {
                            Err(field)
                        } else {
                            Err(order)
                        }
                    },
                    _ => Err(inval),
                }
            }
            _ => unreachable!()
        }
    }
}

impl Range {
    pub fn from_list(list: &str) -> Result<Vec<Range>, String> {
        use std::cmp::max;

        let mut ranges : Vec<Range> = vec!();

        for item in list.split(',') {
            match FromStr::from_str(item) {
                Ok(range_item) => ranges.push(range_item),
                Err(e)=> return Err(format!("range '{}' was invalid: {}", item, e))
            }
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
                if last.high < usize::MAX - 1 {
                    complements.push(Range {
                                        low: last.high + 1,
                                        high: usize::MAX - 1
                                     });
                }
            }
            _ => break
        }
    }

    complements
}
