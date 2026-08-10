// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::ffi::{OsStr, OsString};
use thiserror::Error;
use uucore::translate;

/// Represents an error encountered while parsing a test expression
#[derive(Error, Debug)]
pub enum ParseErrorKind {
    #[error("{}", translate!("test-error-expected-value"))]
    ExpectedValue,
    #[error("{}", translate!("test-error-expected", "value" => .0))]
    Expected(String),
    #[error("{}", translate!("test-error-extra-argument", "argument" => .0))]
    ExtraArgument(String),
    #[error("{}", translate!("test-error-missing-argument", "argument" => .0))]
    MissingArgument(String),
    #[error("{}", translate!("test-error-unknown-operator", "operator" => .0))]
    UnknownOperator(String),
    #[error("{}", translate!("test-error-invalid-integer", "value" => .0))]
    InvalidInteger(String),
    #[error("{}", translate!("test-error-unary-operator-expected", "operator" => .0))]
    UnaryOperatorExpected(String),
}

/// Where in the original argument list an error occurred.
///
/// Only read when a source snippet is rendered.
#[derive(Debug, Default)]
pub enum ErrorAt {
    /// No position could be attributed to the error.
    #[default]
    Unknown,
    /// Zero-based index into the arguments handed to the parser.
    Token(usize),
    /// The first argument equal to this value.
    Value(OsString),
}

/// A parse or evaluation error, together with the position it points at.
#[derive(Error, Debug)]
#[error("{kind}")]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub at: ErrorAt,
}

impl From<ParseErrorKind> for ParseError {
    fn from(kind: ParseErrorKind) -> Self {
        Self {
            kind,
            at: ErrorAt::Unknown,
        }
    }
}

impl ParseError {
    /// An error pointing at the argument with index `index`.
    pub fn at_token(kind: ParseErrorKind, index: usize) -> Self {
        Self {
            kind,
            at: ErrorAt::Token(index),
        }
    }

    /// An error pointing at the first argument equal to `token`.
    pub fn at_value(kind: ParseErrorKind, token: &OsStr) -> Self {
        Self {
            kind,
            at: ErrorAt::Value(token.to_os_string()),
        }
    }
}

/// A Result type for parsing test expressions
pub type ParseResult<T> = Result<T, ParseError>;

/// Implement `UError` trait for `ParseError` to make it easier to return useful error codes from `main()`.
impl uucore::error::UError for ParseError {
    fn code(&self) -> i32 {
        2
    }
}
