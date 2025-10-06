// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore numberparse
//! Errors returned by seq.
use crate::numberparse::ParseNumberError;
use thiserror::Error;
use uucore::display::Quotable;
use uucore::error::UError;
use uucore::translate;

#[derive(Debug, Error)]
pub enum SeqError {
    /// An error parsing the input arguments.
    ///
    /// The parameters are the [`String`] argument as read from the
    /// command line and the underlying parsing error itself.
    #[error("{}", translate!("seq-error-parse", "type" => parse_error_type(.1), "arg" => .0.quote()))]
    ParseError(String, ParseNumberError),

    /// The increment argument was zero, which is not allowed.
    ///
    /// The parameter is the increment argument as a [`String`] as read
    /// from the command line.
    #[error("{}", translate!("seq-error-zero-increment", "arg" => .0.quote()))]
    ZeroIncrement(String),

    /// No arguments were passed to this function, 1 or more is required
    #[error("{}", translate!("seq-error-no-arguments"))]
    NoArguments,

    /// Both a format and equal width where passed to seq
    #[error(
        "{}",
        translate!("seq-error-format-and-equal-width")
    )]
    FormatAndEqualWidth,
}

fn parse_error_type(e: &ParseNumberError) -> String {
    match e {
        ParseNumberError::Float => translate!("seq-parse-error-type-float"),
        ParseNumberError::Nan => translate!("seq-parse-error-type-nan"),
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
