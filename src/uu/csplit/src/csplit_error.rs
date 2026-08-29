// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::io;
use thiserror::Error;
use uucore::display::Quotable;
use uucore::error::{UError, strip_errno};
use uucore::translate;

/// Errors thrown by the csplit command
#[derive(Debug, Error)]
pub enum CsplitError {
    #[error("{}", strip_errno(_0))]
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

#[cfg(test)]
mod tests {
    #[cfg(unix)]
    use super::CsplitError;

    #[cfg(unix)]
    #[test]
    fn io_error_display_is_clean() {
        // GNU does not print "IO error:" nor the raw "(os error N)" suffix.
        let err = CsplitError::IoError(std::io::Error::from_raw_os_error(13));
        let msg = err.to_string();
        assert_eq!(msg, "Permission denied");
        assert!(!msg.contains("IO error:"));
        assert!(!msg.contains("os error"));
    }
}
