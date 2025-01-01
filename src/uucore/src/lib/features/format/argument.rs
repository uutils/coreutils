// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use crate::{
    error::set_exit_code,
    parser::num_parser::{ExtendedParser, ExtendedParserError},
    quoting_style::{Quotes, QuotingStyle, escape_name},
    show_error, show_warning,
};
use os_display::Quotable;
use std::ffi::OsStr;

use super::ExtendedBigDecimal;

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
    Float(ExtendedBigDecimal),
    /// Special argument that gets coerced into the other variants
    Unparsed(String),
}

pub trait ArgumentIter<'a>: Iterator<Item = &'a FormatArgument> {
    fn get_char(&mut self) -> u8;
    fn get_i64(&mut self) -> i64;
    fn get_u64(&mut self) -> u64;
    fn get_extended_big_decimal(&mut self) -> ExtendedBigDecimal;
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
            FormatArgument::Unparsed(s) => {
                // Check if the string is a character literal enclosed in quotes
                if s.starts_with(['"', '\'']) && s.len() > 2 {
                    // Extract the content between the quotes safely
                    let chars: Vec<char> =
                        s.trim_matches(|c| c == '"' || c == '\'').chars().collect();
                    if chars.len() == 1 {
                        return chars[0] as u64; // Return the Unicode code point
                    }
                }
                extract_value(u64::extended_parse(s), s)
            }
            _ => 0,
        }
    }

    fn get_i64(&mut self) -> i64 {
        let Some(next) = self.next() else {
            return 0;
        };
        match next {
            FormatArgument::SignedInt(n) => *n,
            FormatArgument::Unparsed(s) => extract_value(i64::extended_parse(s), s),
            _ => 0,
        }
    }

    fn get_extended_big_decimal(&mut self) -> ExtendedBigDecimal {
        let Some(next) = self.next() else {
            return ExtendedBigDecimal::zero();
        };
        match next {
            FormatArgument::Float(n) => n.clone(),
            FormatArgument::Unparsed(s) => extract_value(ExtendedBigDecimal::extended_parse(s), s),
            _ => ExtendedBigDecimal::zero(),
        }
    }

    fn get_str(&mut self) -> &'a str {
        match self.next() {
            Some(FormatArgument::Unparsed(s) | FormatArgument::String(s)) => s,
            _ => "",
        }
    }
}

fn extract_value<T: Default>(p: Result<T, ExtendedParserError<'_, T>>, input: &str) -> T {
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
                ExtendedParserError::Overflow(v) => {
                    show_error!("{}: Numerical result out of range", input.quote());
                    v
                }
                ExtendedParserError::Underflow(v) => {
                    show_error!("{}: Numerical result out of range", input.quote());
                    v
                }
                ExtendedParserError::NotNumeric => {
                    show_error!("{}: expected a numeric value", input.quote());
                    Default::default()
                }
                ExtendedParserError::PartialMatch(v, rest) => {
                    let bytes = input.as_encoded_bytes();
                    if !bytes.is_empty() && (bytes[0] == b'\'' || bytes[0] == b'"') {
                        show_warning!(
                            "{rest}: character(s) following character constant have been ignored"
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
