// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::fmt;

use crate::string_parser;

/// An error returned when string arg splitting fails.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParseError {
    MissingClosingQuote {
        pos: usize,
        c: char,
    },
    InvalidBackslashAtEndOfStringInMinusS {
        pos: usize,
        quoting: String,
    },
    BackslashCNotAllowedInDoubleQuotes {
        pos: usize,
    },
    InvalidSequenceBackslashXInMinusS {
        pos: usize,
        c: char,
    },
    ParsingOfVariableNameFailed {
        pos: usize,
        msg: String,
    },
    InternalError {
        pos: usize,
        sub_err: string_parser::Error,
    },
    ReachedEnd,
    ContinueWithDelimiter,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(format!("{:?}", self).as_str())
    }
}

impl std::error::Error for ParseError {}

impl From<string_parser::Error> for ParseError {
    fn from(value: string_parser::Error) -> Self {
        Self::InternalError {
            pos: value.peek_position,
            sub_err: value,
        }
    }
}
