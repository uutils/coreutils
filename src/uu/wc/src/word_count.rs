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
