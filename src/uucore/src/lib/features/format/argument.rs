// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use super::FormatError;
use crate::{
    error::{set_exit_code, UError},
    features::format::num_parser::{ParseError, ParsedNumber},
    quoting_style::{escape_name, Quotes, QuotingStyle},
    show_error, show_warning,
};
use os_display::Quotable;
use std::{
    error::Error,
    ffi::{OsStr, OsString},
    fmt::Display,
};

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
    fn get_char(&mut self) -> Result<u8, FormatError>;
    fn get_i64(&mut self) -> Result<i64, FormatError>;
    fn get_u64(&mut self) -> Result<u64, FormatError>;
    fn get_f64(&mut self) -> Result<f64, FormatError>;
    fn get_str(&mut self) -> &'a OsStr;
}

impl<'a, T: Iterator<Item = &'a FormatArgument>> ArgumentIter<'a> for T {
    fn get_char(&mut self) -> Result<u8, FormatError> {
        let Some(next) = self.next() else {
            return Ok(b'\0');
        };
        match next {
            FormatArgument::Char(c) => Ok(*c as u8),
            FormatArgument::Unparsed(os) => match try_get_bytes_from_os_str(os)?.first() {
                Some(&byte) => Ok(byte),
                None => Ok(b'\0'),
            },
            _ => Ok(b'\0'),
        }
    }

    fn get_u64(&mut self) -> Result<u64, FormatError> {
        let Some(next) = self.next() else {
            return Ok(0);
        };
        match next {
            FormatArgument::UnsignedInt(n) => Ok(*n),
            FormatArgument::Unparsed(os) => {
                let str = try_get_str_from_os_str(os)?;

                Ok(extract_value(ParsedNumber::parse_u64(str), str))
            }
            _ => Ok(0),
        }
    }

    fn get_i64(&mut self) -> Result<i64, FormatError> {
        let Some(next) = self.next() else {
            return Ok(0);
        };
        match next {
            FormatArgument::SignedInt(n) => Ok(*n),
            FormatArgument::Unparsed(os) => {
                let str = try_get_str_from_os_str(os)?;

                Ok(extract_value(ParsedNumber::parse_i64(str), str))
            }
            _ => Ok(0),
        }
    }

    fn get_f64(&mut self) -> Result<f64, FormatError> {
        let Some(next) = self.next() else {
            return Ok(0.0);
        };
        match next {
            FormatArgument::Float(n) => Ok(*n),
            FormatArgument::Unparsed(os) => {
                let str = try_get_str_from_os_str(os)?;

                Ok(extract_value(ParsedNumber::parse_f64(str), str))
            }
            _ => Ok(0.0),
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

#[derive(Debug)]
pub struct NonUtf8OsStr(pub String);

impl Display for NonUtf8OsStr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "invalid (non-UTF-8) string like {} encountered",
            self.0.quote(),
        ))
    }
}

impl Error for NonUtf8OsStr {}
impl UError for NonUtf8OsStr {}

pub fn try_get_str_from_os_str(os_str: &OsStr) -> Result<&str, NonUtf8OsStr> {
    match os_str.to_str() {
        Some(st) => Ok(st),
        None => {
            let cow = os_str.to_string_lossy();

            Err(NonUtf8OsStr(cow.to_string()))
        }
    }
}

pub fn try_get_bytes_from_os_str(input: &OsStr) -> Result<&[u8], NonUtf8OsStr> {
    let result = {
        #[cfg(target_family = "unix")]
        {
            use std::os::unix::ffi::OsStrExt;

            Ok(input.as_bytes())
        }

        #[cfg(not(target_family = "unix"))]
        {
            // TODO
            // Verify that this works correctly on these platforms
            match input.to_str().map(|st| st.as_bytes()) {
                Some(sl) => Ok(sl),
                None => {
                    let cow = input.to_string_lossy();

                    Err(NonUtf8OsStr(cow.to_string()))
                }
            }
        }
    };

    result
}
