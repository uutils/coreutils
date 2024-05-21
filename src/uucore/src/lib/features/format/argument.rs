// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use crate::{
    error::set_exit_code,
    features::format::num_parser::{ParseError, ParsedNumber},
    quoting_style::{escape_name, Quotes, QuotingStyle},
    show_error, show_warning,
};
use os_display::Quotable;
use std::ffi::OsStr;

/// An argument for formatting
///
/// Each of these variants is only accepted by their respective directives. For
/// example, [`FormatArgument::Char`] requires a `%c` directive.
///
/// The [`FormatArgument::Unparsed`] variant contains a string that can be
/// parsed into other types. This is used by the `printf` utility.
#[derive(Clone, Debug)]
pub enum FormatArgument {
    Char(char),
    String(String),
    UnsignedInt(u64),
    SignedInt(i64),
    Float(f64),
    /// Special argument that gets coerced into the other variants
    Unparsed(String),
}

pub trait ArgumentIter<'a>: Iterator<Item = &'a FormatArgument> {
    fn get_char(&mut self) -> u8;
    fn get_i64(&mut self) -> i64;
    fn get_u64(&mut self) -> u64;
    fn get_f64(&mut self) -> f64;
    fn get_str(&mut self) -> &'a str;
}

impl<'a, T: Iterator<Item = &'a FormatArgument>> ArgumentIter<'a> for T {
    fn get_char(&mut self) -> u8 {
        let Some(next) = self.next() else {
            return b'\0';
        };
        match next {
            FormatArgument::Char(c) => *c as u8,
            FormatArgument::Unparsed(s) => s.bytes().next().unwrap_or(b'\0'),
            _ => b'\0',
        }
    }

    fn get_u64(&mut self) -> u64 {
        let Some(next) = self.next() else {
            return 0;
        };
        match next {
            FormatArgument::UnsignedInt(n) => *n,
            FormatArgument::Unparsed(s) => extract_value(ParsedNumber::parse_u64(s), s),
            _ => 0,
        }
    }

    fn get_i64(&mut self) -> i64 {
        let Some(next) = self.next() else {
            return 0;
        };
        match next {
            FormatArgument::SignedInt(n) => *n,
            FormatArgument::Unparsed(s) => extract_value(ParsedNumber::parse_i64(s), s),
            _ => 0,
        }
    }

    fn get_f64(&mut self) -> f64 {
        let Some(next) = self.next() else {
            return 0.0;
        };
        match next {
            FormatArgument::Float(n) => *n,
            FormatArgument::Unparsed(s) => extract_value(ParsedNumber::parse_f64(s), s),
            _ => 0.0,
        }
    }

    fn get_str(&mut self) -> &'a str {
        match self.next() {
            Some(FormatArgument::Unparsed(s) | FormatArgument::String(s)) => s,
            _ => "",
        }
    }
}

fn extract_value<T: Default>(p: Result<T, ParseError<'_, T>>, input: &str) -> T {
    match p {
        Ok(v) => v,
        Err(e) => {
            set_exit_code(1);
            let input = escape_name(
                OsStr::new(input),
                &QuotingStyle::C {
                    quotes: Quotes::None,
                },
            );
            match e {
                ParseError::Overflow => {
                    show_error!("{}: Numerical result out of range", input.quote());
                    Default::default()
                }
                ParseError::NotNumeric => {
                    show_error!("{}: expected a numeric value", input.quote());
                    Default::default()
                }
                ParseError::PartialMatch(v, rest) => {
                    if input.starts_with('\'') {
                        show_warning!(
                            "{}: character(s) following character constant have been ignored",
                            &rest,
                        );
                    } else {
                        show_error!("{}: value not completely converted", input.quote());
                    }
                    v
                }
            }
        }
    }
}
