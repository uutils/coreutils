use std::io;

/// Errors thrown by the csplit command
#[derive(Debug, Fail)]
pub enum CsplitError {
    #[fail(display = "IO error: {}", _0)]
    IoError(io::Error),
    #[fail(display = "'{}': line number out of range", _0)]
    LineOutOfRange(String),
    #[fail(display = "'{}': line number out of range on repetition {}", _0, _1)]
    LineOutOfRangeOnRepetition(String, usize),
    #[fail(display = "'{}': match not found", _0)]
    MatchNotFound(String),
    #[fail(display = "'{}': match not found on repetition {}", _0, _1)]
    MatchNotFoundOnRepetition(String, usize),
    #[fail(display = "line number must be greater than zero")]
    LineNumberIsZero,
    #[fail(display = "line number '{}' is smaller than preceding line number, {}", _0, _1)]
    LineNumberSmallerThanPrevious(usize, usize),
    #[fail(display = "invalid pattern: {}", _0)]
    InvalidPattern(String),
    #[fail(display = "invalid number: '{}'", _0)]
    InvalidNumber(String),
    #[fail(display = "incorrect conversion specification in suffix")]
    SuffixFormatIncorrect,
    #[fail(display = "too many % conversion specifications in suffix")]
    SuffixFormatTooManyPercents,
}

impl From<io::Error> for CsplitError {
    fn from(error: io::Error) -> Self {
        CsplitError::IoError(error)
    }
}