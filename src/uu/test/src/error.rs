// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use thiserror::Error;
use uucore::translate;

/// Represents an error encountered while parsing a test expression
#[derive(Error, Debug)]
pub enum ParseError {
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

/// A Result type for parsing test expressions
pub type ParseResult<T> = Result<T, ParseError>;

/// Implement `UError` trait for `ParseError` to make it easier to return useful error codes from `main()`.
impl uucore::error::UError for ParseError {
    fn code(&self) -> i32 {
        2
    }
}
