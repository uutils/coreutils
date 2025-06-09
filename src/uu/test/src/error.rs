// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use thiserror::Error;

/// Represents an error encountered while parsing a test expression
#[derive(Error, Debug)]
pub enum ParseError {
    #[error("expected value")]
    ExpectedValue,
    #[error("expected {0}")]
    Expected(String),
    #[error("extra argument {0}")]
    ExtraArgument(String),
    #[error("missing argument after {0}")]
    MissingArgument(String),
    #[error("unknown operator {0}")]
    UnknownOperator(String),
    #[error("invalid integer {0}")]
    InvalidInteger(String),
    #[error("{0}: unary operator expected")]
    UnaryOperatorExpected(String),
}

/// A Result type for parsing test expressions
pub type ParseResult<T> = Result<T, ParseError>;

/// Implement UError trait for ParseError to make it easier to return useful error codes from main().
impl uucore::error::UError for ParseError {
    fn code(&self) -> i32 {
        2
    }
}
