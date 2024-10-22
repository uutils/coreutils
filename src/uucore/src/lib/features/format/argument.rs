// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use crate::{
    error::{set_exit_code, UResult, USimpleError},
    features::format::num_parser::{ParseError, ParsedNumber},
    quoting_style::{escape_name, Quotes, QuotingStyle},
    show, show_error, show_warning,
};
use os_display::Quotable;
use std::ffi::{OsStr, OsString};

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
    String(OsString),
    UnsignedInt(u64),
    SignedInt(i64),
    Float(f64),
    /// Special argument that gets coerced into the other variants
    Unparsed(OsString),
}

pub trait ArgumentIter<'a>: Iterator<Item = &'a FormatArgument> {
    fn get_char(&mut self) -> u8;
    fn get_i64(&mut self) -> i64;
    fn get_u64(&mut self) -> u64;
    fn get_f64(&mut self) -> f64;
    fn get_str(&mut self) -> &'a OsStr;
}

impl<'a, T: Iterator<Item = &'a FormatArgument>> ArgumentIter<'a> for T {
    fn get_char(&mut self) -> u8 {
        let Some(next) = self.next() else {
            return b'\0';
        };
        match next {
            FormatArgument::Char(c) => *c as u8,
            FormatArgument::Unparsed(os) => match bytes_from_os_str(os).unwrap().first() {
                Some(&byte) => byte,
                None => b'\0',
            },
            _ => b'\0',
        }
    }

    fn get_u64(&mut self) -> u64 {
        let Some(next) = self.next() else {
            return 0;
        };
        match next {
            FormatArgument::UnsignedInt(n) => *n,
            FormatArgument::Unparsed(os) => {
                let s = get_str_or_exit_with_error(os);
                // Check if the string is a character literal enclosed in quotes
                if s.starts_with(['"', '\'']) {
                    // Extract the content between the quotes safely using chars
                    let mut chars = s.trim_matches(|c| c == '"' || c == '\'').chars();
                    if let Some(first_char) = chars.next() {
                        if chars.clone().count() > 0 {
                            // Emit a warning if there are additional characters
                            let remaining: String = chars.collect();
                            show_warning!(
                                "{}: character(s) following character constant have been ignored",
                                remaining
                            );
                        }
                        return first_char as u64; // Use only the first character
                    }
                    return 0; // Empty quotes
                }
                extract_value(ParsedNumber::parse_u64(s), s)
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
            FormatArgument::Unparsed(os) => {
                let str = get_str_or_exit_with_error(os);

                extract_value(ParsedNumber::parse_i64(str), str)
            }
            _ => 0,
        }
    }

    fn get_f64(&mut self) -> f64 {
        let Some(next) = self.next() else {
            return 0.0;
        };
        match next {
            FormatArgument::Float(n) => *n,
            FormatArgument::Unparsed(os) => {
                let str = get_str_or_exit_with_error(os);

                extract_value(ParsedNumber::parse_f64(str), str)
            }
            _ => 0.0,
        }
    }

    fn get_str(&mut self) -> &'a OsStr {
        match self.next() {
            Some(FormatArgument::Unparsed(os) | FormatArgument::String(os)) => os,
            _ => "".as_ref(),
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
                    let bytes = input.as_encoded_bytes();
                    if !bytes.is_empty() && bytes[0] == b'\'' {
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

pub fn bytes_from_os_str(input: &OsStr) -> UResult<&[u8]> {
    let result = {
        #[cfg(target_family = "unix")]
        {
            use std::os::unix::ffi::OsStrExt;

            Ok(input.as_bytes())
        }

        #[cfg(not(target_family = "unix"))]
        {
            use crate::error::USimpleError;

            // TODO
            // Verify that this works correctly on these platforms
            match input.to_str().map(|st| st.as_bytes()) {
                Some(sl) => Ok(sl),
                None => Err(USimpleError::new(
                    1,
                    "non-UTF-8 string encountered when not allowed",
                )),
            }
        }
    };

    result
}

fn get_str_or_exit_with_error(os_str: &OsStr) -> &str {
    match os_str.to_str() {
        Some(st) => st,
        None => {
            let cow = os_str.to_string_lossy();

            let quoted = cow.quote();

            let error = format!(
                "argument like {quoted} is not a valid UTF-8 string, and could not be parsed as an integer",
            );

            show!(USimpleError::new(1, error.clone()));

            panic!("{error}");
        }
    }
}
