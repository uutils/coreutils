// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::collections::HashMap;
use std::io;
use thiserror::Error;
use uucore::display::Quotable;
use uucore::error::UError;
use uucore::locale::{get_message, get_message_with_args};

/// Errors thrown by the csplit command
#[derive(Debug, Error)]
pub enum CsplitError {
    #[error("IO error: {}", _0)]
    IoError(#[from] io::Error),
    #[error("{}", get_message_with_args("csplit-error-line-out-of-range", HashMap::from([("pattern".to_string(), _0.quote().to_string())])))]
    LineOutOfRange(String),
    #[error("{}", get_message_with_args("csplit-error-line-out-of-range-on-repetition", HashMap::from([("pattern".to_string(), _0.quote().to_string()), ("repetition".to_string(), _1.to_string())])))]
    LineOutOfRangeOnRepetition(String, usize),
    #[error("{}", get_message_with_args("csplit-error-match-not-found", HashMap::from([("pattern".to_string(), _0.quote().to_string())])))]
    MatchNotFound(String),
    #[error("{}", get_message_with_args("csplit-error-match-not-found-on-repetition", HashMap::from([("pattern".to_string(), _0.quote().to_string()), ("repetition".to_string(), _1.to_string())])))]
    MatchNotFoundOnRepetition(String, usize),
    #[error("{}", get_message("csplit-error-line-number-is-zero"))]
    LineNumberIsZero,
    #[error("{}", get_message_with_args("csplit-error-line-number-smaller-than-previous", HashMap::from([("current".to_string(), _0.to_string()), ("previous".to_string(), _1.to_string())])))]
    LineNumberSmallerThanPrevious(usize, usize),
    #[error("{}", get_message_with_args("csplit-error-invalid-pattern", HashMap::from([("pattern".to_string(), _0.quote().to_string())])))]
    InvalidPattern(String),
    #[error("{}", get_message_with_args("csplit-error-invalid-number", HashMap::from([("number".to_string(), _0.quote().to_string())])))]
    InvalidNumber(String),
    #[error("{}", get_message("csplit-error-suffix-format-incorrect"))]
    SuffixFormatIncorrect,
    #[error("{}", get_message("csplit-error-suffix-format-too-many-percents"))]
    SuffixFormatTooManyPercents,
    #[error("{}", get_message_with_args("csplit-error-not-regular-file", HashMap::from([("file".to_string(), _0.quote().to_string())])))]
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
