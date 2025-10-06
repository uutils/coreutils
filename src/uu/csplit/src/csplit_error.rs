// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::io;
use thiserror::Error;
use uucore::display::Quotable;
use uucore::error::UError;
use uucore::translate;

/// Errors thrown by the csplit command
#[derive(Debug, Error)]
pub enum CsplitError {
    #[error("IO error: {}", _0)]
    IoError(#[from] io::Error),
    #[error("{}", translate!("csplit-error-line-out-of-range", "pattern" => _0.quote()))]
    LineOutOfRange(String),
    #[error("{}", translate!("csplit-error-line-out-of-range-on-repetition", "pattern" => _0.quote(), "repetition" => _1))]
    LineOutOfRangeOnRepetition(String, usize),
    #[error("{}", translate!("csplit-error-match-not-found", "pattern" => _0.quote()))]
    MatchNotFound(String),
    #[error("{}", translate!("csplit-error-match-not-found-on-repetition", "pattern" => _0.quote(), "repetition" => _1))]
    MatchNotFoundOnRepetition(String, usize),
    #[error("{}", translate!("csplit-error-line-number-is-zero"))]
    LineNumberIsZero,
    #[error("{}", translate!("csplit-error-line-number-smaller-than-previous", "current" => _0, "previous" => _1))]
    LineNumberSmallerThanPrevious(usize, usize),
    #[error("{}", translate!("csplit-error-invalid-pattern", "pattern" => _0.quote()))]
    InvalidPattern(String),
    #[error("{}", translate!("csplit-error-invalid-number", "number" => _0.quote()))]
    InvalidNumber(String),
    #[error("{}", translate!("csplit-error-suffix-format-incorrect"))]
    SuffixFormatIncorrect,
    #[error("{}", translate!("csplit-error-suffix-format-too-many-percents"))]
    SuffixFormatTooManyPercents,
    #[error("{}", translate!("csplit-error-not-regular-file", "file" => _0.quote()))]
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
