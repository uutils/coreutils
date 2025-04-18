// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use std::io;
use thiserror::Error;

use uucore::display::Quotable;
use uucore::error::UError;

/// Errors thrown by the csplit command
#[derive(Debug, Error)]
pub enum CsplitError {
    #[error("IO error: {}", _0)]
    IoError(#[from] io::Error),
    #[error("{}: line number out of range", ._0.quote())]
    LineOutOfRange(String),
    #[error("{}: line number out of range on repetition {}", ._0.quote(), _1)]
    LineOutOfRangeOnRepetition(String, usize),
    #[error("{}: match not found", ._0.quote())]
    MatchNotFound(String),
    #[error("{}: match not found on repetition {}", ._0.quote(), _1)]
    MatchNotFoundOnRepetition(String, usize),
    #[error("0: line number must be greater than zero")]
    LineNumberIsZero,
    #[error("line number '{}' is smaller than preceding line number, {}", _0, _1)]
    LineNumberSmallerThanPrevious(usize, usize),
    #[error("{}: invalid pattern", ._0.quote())]
    InvalidPattern(String),
    #[error("invalid number: {}", ._0.quote())]
    InvalidNumber(String),
    #[error("incorrect conversion specification in suffix")]
    SuffixFormatIncorrect,
    #[error("too many % conversion specifications in suffix")]
    SuffixFormatTooManyPercents,
    #[error("{} is not a regular file", ._0.quote())]
    NotRegularFile(String),
    #[error("{}", _0)]
    UError(Box<dyn UError>),
}

impl From<Box<dyn UError>> for CsplitError {
    fn from(error: Box<dyn UError>) -> Self {
        Self::UError(error)
    }
}

impl UError for CsplitError {
    fn code(&self) -> i32 {
        match self {
            Self::UError(e) => e.code(),
            _ => 1,
        }
    }
}
