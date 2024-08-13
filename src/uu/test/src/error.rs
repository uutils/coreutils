// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
/// Represents an error encountered while parsing a test expression
#[derive(Debug)]
pub enum ParseError {
    ExpectedValue,
    Expected(String),
    ExtraArgument(String),
    MissingArgument(String),
    UnknownOperator(String),
    InvalidInteger(String),
    UnaryOperatorExpected(String),
}

/// A Result type for parsing test expressions
pub type ParseResult<T> = Result<T, ParseError>;

/// Implement Display trait for ParseError to make it easier to print useful errors.
impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Expected(s) => write!(f, "expected {s}"),
            Self::ExpectedValue => write!(f, "expected value"),
            Self::MissingArgument(s) => write!(f, "missing argument after {s}"),
            Self::ExtraArgument(s) => write!(f, "extra argument {s}"),
            Self::UnknownOperator(s) => write!(f, "unknown operator {s}"),
            Self::InvalidInteger(s) => write!(f, "invalid integer {s}"),
            Self::UnaryOperatorExpected(op) => write!(f, "{op}: unary operator expected"),
        }
    }
}

/// Implement UError trait for ParseError to make it easier to return useful error codes from main().
impl uucore::error::UError for ParseError {
    fn code(&self) -> i32 {
        2
    }
}

/// Implement standard Error trait for UError
impl std::error::Error for ParseError {}
