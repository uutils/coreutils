// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore numberparse
//! Errors returned by seq.
use crate::numberparse::ParseNumberError;
use std::collections::HashMap;
use thiserror::Error;
use uucore::display::Quotable;
use uucore::error::UError;
use uucore::locale::{get_message, get_message_with_args};

#[derive(Debug, Error)]
pub enum SeqError {
    /// An error parsing the input arguments.
    ///
    /// The parameters are the [`String`] argument as read from the
    /// command line and the underlying parsing error itself.
    #[error("{}", get_message_with_args("seq-error-parse", HashMap::from([("type".to_string(), parse_error_type(.1).to_string()), ("arg".to_string(), .0.quote().to_string())])))]
    ParseError(String, ParseNumberError),

    /// The increment argument was zero, which is not allowed.
    ///
    /// The parameter is the increment argument as a [`String`] as read
    /// from the command line.
    #[error("{}", get_message_with_args("seq-error-zero-increment", HashMap::from([("arg".to_string(), .0.quote().to_string())])))]
    ZeroIncrement(String),

    /// No arguments were passed to this function, 1 or more is required
    #[error("{}", get_message_with_args("seq-error-no-arguments", HashMap::new()))]
    NoArguments,

    /// Both a format and equal width where passed to seq
    #[error(
        "{}",
        get_message_with_args("seq-error-format-and-equal-width", HashMap::new())
    )]
    FormatAndEqualWidth,
}

fn parse_error_type(e: &ParseNumberError) -> String {
    match e {
        ParseNumberError::Float => get_message("seq-parse-error-type-float"),
        ParseNumberError::Nan => get_message("seq-parse-error-type-nan"),
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
