//  * This file is part of the uutils coreutils package.
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.
// spell-checker:ignore numberparse argtype
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
}

impl SeqError {
    /// The [`String`] argument as read from the command-line.
    fn arg(&self) -> &str {
        match self {
            Self::ParseError(s, _) => s,
            Self::ZeroIncrement(s) => s,
        }
    }

    /// The type of argument that is causing the error.
    fn argtype(&self) -> &str {
        match self {
            Self::ParseError(_, e) => match e {
                ParseNumberError::Float => "floating point argument",
                ParseNumberError::Nan => "'not-a-number' argument",
                ParseNumberError::Hex => "hexadecimal argument",
            },
            Self::ZeroIncrement(_) => "Zero increment value",
        }
    }
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
        write!(f, "invalid {}: {}", self.argtype(), self.arg().quote())
    }
}
