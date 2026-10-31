// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::cmp::max;

#[derive(Debug, Default, Copy, Clone)]
pub struct WordCount {
    pub bytes: u64,
    pub chars: u64,
    pub lines: u64,
    pub words: u64,
    pub max_line_length: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct CountOverflow {
    pub bytes: bool,
    pub chars: bool,
    pub lines: bool,
    pub words: bool,
}

impl WordCount {
    /// Accumulate counts, saturating each independently and retaining overflow flags.
    pub fn accumulate(&mut self, other: Self, overflow: &mut CountOverflow) {
        for (total, count, overflowed) in [
            (&mut self.bytes, other.bytes, &mut overflow.bytes),
            (&mut self.chars, other.chars, &mut overflow.chars),
            (&mut self.lines, other.lines, &mut overflow.lines),
            (&mut self.words, other.words, &mut overflow.words),
        ] {
            if let Some(sum) = total.checked_add(count) {
                *total = sum;
            } else {
                *total = u64::MAX;
                *overflowed = true;
            }
        }
        self.max_line_length = max(self.max_line_length, other.max_line_length);
    }
}

#[cfg(test)]
mod tests {
    use super::{CountOverflow, WordCount};

    #[test]
    fn accumulate_saturates_each_count_independently() {
        let mut total = WordCount {
            bytes: u64::MAX - 1,
            chars: u64::MAX - 2,
            lines: u64::MAX - 3,
            words: u64::MAX - 4,
            max_line_length: 20,
        };
        let mut overflow = CountOverflow::default();
        total.accumulate(
            WordCount {
                bytes: 1,
                chars: 2,
                lines: 3,
                words: 4,
                max_line_length: 10,
            },
            &mut overflow,
        );
        assert_eq!(overflow, CountOverflow::default());
        total.accumulate(
            WordCount {
                bytes: 1,
                ..WordCount::default()
            },
            &mut overflow,
        );
        assert_eq!(
            overflow,
            CountOverflow {
                bytes: true,
                ..CountOverflow::default()
            }
        );
        total.accumulate(
            WordCount {
                chars: 1,
                lines: 1,
                words: 1,
                ..WordCount::default()
            },
            &mut overflow,
        );
        assert_eq!(
            overflow,
            CountOverflow {
                bytes: true,
                chars: true,
                lines: true,
                words: true
            }
        );
        assert_eq!(
            (total.bytes, total.chars, total.lines, total.words),
            (u64::MAX, u64::MAX, u64::MAX, u64::MAX)
        );
        assert_eq!(total.max_line_length, 20);
    }

    #[test]
    fn accumulate_keeps_other_counts_after_overflow() {
        let mut total = WordCount {
            bytes: u64::MAX,
            ..WordCount::default()
        };
        let mut overflow = CountOverflow::default();
        total.accumulate(
            WordCount {
                bytes: 1,
                chars: 3,
                lines: 2,
                words: 1,
                max_line_length: 5,
            },
            &mut overflow,
        );
        assert_eq!(
            overflow,
            CountOverflow {
                bytes: true,
                ..CountOverflow::default()
            }
        );
        assert_eq!(
            (total.chars, total.lines, total.words, total.max_line_length),
            (3, 2, 1, 5)
        );
    }
}
