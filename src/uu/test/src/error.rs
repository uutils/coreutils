// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::collections::HashMap;
use thiserror::Error;
use uucore::locale::{get_message, get_message_with_args};

/// Represents an error encountered while parsing a test expression
#[derive(Error, Debug)]
pub enum ParseError {
    #[error("{}", get_message("test-error-expected-value"))]
    ExpectedValue,
    #[error("{}", get_message_with_args("test-error-expected", HashMap::from([("value".to_string(), .0.to_string())])))]
    Expected(String),
    #[error("{}", get_message_with_args("test-error-extra-argument", HashMap::from([("argument".to_string(), .0.to_string())])))]
    ExtraArgument(String),
    #[error("{}", get_message_with_args("test-error-missing-argument", HashMap::from([("argument".to_string(), .0.to_string())])))]
    MissingArgument(String),
    #[error("{}", get_message_with_args("test-error-unknown-operator", HashMap::from([("operator".to_string(), .0.to_string())])))]
    UnknownOperator(String),
    #[error("{}", get_message_with_args("test-error-invalid-integer", HashMap::from([("value".to_string(), .0.to_string())])))]
    InvalidInteger(String),
    #[error("{}", get_message_with_args("test-error-unary-operator-expected", HashMap::from([("operator".to_string(), .0.to_string())])))]
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
