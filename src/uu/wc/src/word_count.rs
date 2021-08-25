use std::cmp::max;
use std::iter::Sum;
use std::ops::{Add, AddAssign};
use std::str::from_utf8;

const CR: u8 = b'\r';
const LF: u8 = b'\n';
const SPACE: u8 = b' ';
const TAB: u8 = b'\t';
const SYN: u8 = 0x16_u8;
const FF: u8 = 0x0C_u8;

#[inline(always)]
fn is_word_separator(byte: u8) -> bool {
    byte == SPACE || byte == TAB || byte == CR || byte == SYN || byte == FF
}

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
        *self = *self + other
    }
}

impl Sum for WordCount {
    fn sum<I>(iter: I) -> WordCount
    where
        I: Iterator<Item = WordCount>,
    {
        iter.fold(WordCount::default(), |acc, x| acc + x)
    }
}

impl WordCount {
    /// Count the characters and whitespace-separated words in the given bytes.
    ///
    /// `line` is a slice of bytes that will be decoded as ASCII characters.
    fn ascii_word_and_char_count(line: &[u8]) -> (usize, usize) {
        let word_count = line.split(|&x| is_word_separator(x)).count();
        let char_count = line.iter().filter(|c| c.is_ascii()).count();
        (word_count, char_count)
    }

    /// Create a [`WordCount`] from a sequence of bytes representing a line.
    ///
    /// If the last byte of `line` encodes a newline character (`\n`),
    /// then the [`lines`] field will be set to 1. Otherwise, it will
    /// be set to 0. The [`bytes`] field is simply the length of
    /// `line`.
    ///
    /// If `decode_chars` is `false`, the [`chars`] and [`words`]
    /// fields will be set to 0. If it is `true`, this function will
    /// attempt to decode the bytes first as UTF-8, and failing that,
    /// as ASCII.
    pub fn from_line(line: &[u8]) -> WordCount {
        // GNU 'wc' only counts lines that end in LF as lines
        let lines = (*line.last().unwrap() == LF) as usize;
        let bytes = line.len();
        let (words, chars) = WordCount::word_and_char_count(line);
        // -L is a GNU 'wc' extension so same behavior on LF
        let max_line_length = if chars > 0 { chars - lines } else { 0 };
        WordCount {
            bytes,
            chars,
            lines,
            words,
            max_line_length,
        }
    }

    /// Count the UTF-8 characters and words in the given string slice.
    ///
    /// `s` is a string slice that is assumed to be a UTF-8 string.
    fn utf8_word_and_char_count(s: &str) -> (usize, usize) {
        let word_count = s.split_whitespace().count();
        let char_count = s.chars().count();
        (word_count, char_count)
    }

    pub fn with_title(self, title: Option<&str>) -> TitledWordCount {
        TitledWordCount { title, count: self }
    }

    /// Count the characters and words in the given slice of bytes.
    ///
    /// `line` is a slice of bytes that will be decoded as UTF-8
    /// characters, or if that fails, as ASCII characters.
    fn word_and_char_count(line: &[u8]) -> (usize, usize) {
        // try and convert the bytes to UTF-8 first
        match from_utf8(line) {
            Ok(s) => WordCount::utf8_word_and_char_count(s),
            Err(..) => WordCount::ascii_word_and_char_count(line),
        }
    }
}

/// This struct supplements the actual word count with an optional title that is
/// displayed to the user at the end of the program.
/// The reason we don't simply include title in the `WordCount` struct is that
/// it would result in unnecessary copying of `String`.
#[derive(Debug, Default, Clone)]
pub struct TitledWordCount<'a> {
    pub title: Option<&'a str>,
    pub count: WordCount,
}
