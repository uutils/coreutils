// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore numberparse
//! Errors returned by seq.
use std::error::Error;
use std::fmt::Display;

use uucore::display::Quotable;
use uucore::error::UError;

use crate::numberparse::ParseNumberError;

#[derive(Debug)]
pub enum SeqError {
    /// An error parsing the input arguments.
    ///
    /// The parameters are the [`String`] argument as read from the
    /// command line and the underlying parsing error itself.
    ParseError(String, ParseNumberError),

    /// The increment argument was zero, which is not allowed.
    ///
    /// The parameter is the increment argument as a [`String`] as read
    /// from the command line.
    ZeroIncrement(String),

    /// No arguments were passed to this function, 1 or more is required
    NoArguments,
}

impl UError for SeqError {
    /// Always return 1.
    fn code(&self) -> i32 {
        1
    }

    fn usage(&self) -> bool {
        true
    }
}

impl Error for SeqError {}

impl Display for SeqError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ParseError(s, e) => {
                let error_type = match e {
                    ParseNumberError::Float => "floating point",
                    ParseNumberError::Nan => "'not-a-number'",
                    ParseNumberError::Hex => "hexadecimal",
                };
                write!(f, "invalid {error_type} argument: {}", s.quote())
            }
            Self::ZeroIncrement(s) => write!(f, "invalid Zero increment value: {}", s.quote()),
            Self::NoArguments => write!(f, "missing operand"),
        }
    }
}
