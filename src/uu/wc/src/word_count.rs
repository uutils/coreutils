use std::cmp::max;
use std::ops::{Add, AddAssign};
use std::path::Path;

#[derive(Debug, Default, Copy, Clone)]
pub struct WordCount {
    pub bytes: usize,
    pub chars: usize,
    pub lines: usize,
    pub words: usize,
    pub max_line_length: usize,
}

impl Add for WordCount {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            bytes: self.bytes + other.bytes,
            chars: self.chars + other.chars,
            lines: self.lines + other.lines,
            words: self.words + other.words,
            max_line_length: max(self.max_line_length, other.max_line_length),
        }
    }
}

impl AddAssign for WordCount {
    fn add_assign(&mut self, other: Self) {
        *self = *self + other;
    }
}

impl WordCount {
    pub fn with_title(self, title: Option<&Path>) -> TitledWordCount {
        TitledWordCount { title, count: self }
    }
}

/// This struct supplements the actual word count with an optional title that is
/// displayed to the user at the end of the program.
/// The reason we don't simply include title in the `WordCount` struct is that
/// it would result in unnecessary copying of `String`.
#[derive(Debug, Default, Clone)]
pub struct TitledWordCount<'a> {
    pub title: Option<&'a Path>,
    pub count: WordCount,
}
