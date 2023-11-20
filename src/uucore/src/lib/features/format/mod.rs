//! Main entry point for our implementation of printf.
//!
//! The [`printf`] and [`sprintf`] closely match the behavior of the
//! corresponding C functions: the former renders a formatted string
//! to stdout, the latter renders to a new [`String`] object.
//!
//! In addition to the [`printf`] and [`sprintf`] functions, we expose the
//! [`Format`] struct, which represents a parsed format string. This reduces
//! the need for parsing a format string multiple times and assures that no
//! parsing errors occur during writing.
//!
//! There are three kinds of parsing that we might want to do:
//!
//!  1. Only `printf` specifiers (for e.g. `seq`, `dd`)
//!  2. Only escape sequences (for e.g. `echo`)
//!  3. Both `printf` specifiers and escape sequences (for e.g. `printf`)
//!
//! This module aims to combine all three use cases.

// spell-checker:ignore (vars) charf decf floatf intf scif strf Cninety

mod argument;
mod escape;
pub mod num_format;
mod spec;

pub use argument::*;
use spec::Spec;
use std::{
    error::Error,
    fmt::Display,
    io::{stdout, Write},
    ops::ControlFlow,
};

use crate::error::UError;

use self::{
    escape::{parse_escape_code, EscapedChar},
    num_format::Formatter,
};

#[derive(Debug)]
pub enum FormatError {
    SpecError,
    IoError(std::io::Error),
    NoMoreArguments,
    InvalidArgument(FormatArgument),
}

impl Error for FormatError {}
impl UError for FormatError {}

impl From<std::io::Error> for FormatError {
    fn from(value: std::io::Error) -> Self {
        FormatError::IoError(value)
    }
}

impl Display for FormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO: Be more precise about these
        match self {
            FormatError::SpecError => write!(f, "invalid spec"),
            FormatError::IoError(_) => write!(f, "io error"),
            FormatError::NoMoreArguments => write!(f, "no more arguments"),
            FormatError::InvalidArgument(_) => write!(f, "invalid argument"),
        }
    }
}

/// A single item to format
pub enum FormatItem<C: FormatChar> {
    /// A format specifier
    Spec(Spec),
    /// A single character
    Char(C),
}

pub trait FormatChar {
    fn write(&self, writer: impl Write) -> std::io::Result<ControlFlow<()>>;
}

impl FormatChar for u8 {
    fn write(&self, mut writer: impl Write) -> std::io::Result<ControlFlow<()>> {
        writer.write(&[*self])?;
        Ok(ControlFlow::Continue(()))
    }
}

impl FormatChar for EscapedChar {
    fn write(&self, mut writer: impl Write) -> std::io::Result<ControlFlow<()>> {
        match self {
            EscapedChar::Byte(c) => {
                writer.write(&[*c])?;
            }
            EscapedChar::Char(c) => {
                write!(writer, "{c}")?;
            }
            EscapedChar::Backslash(c) => {
                writer.write(&[b'\\', *c])?;
            }
            EscapedChar::End => return Ok(ControlFlow::Break(())),
        }
        Ok(ControlFlow::Continue(()))
    }
}

impl<C: FormatChar> FormatItem<C> {
    pub fn write<'a>(
        &self,
        writer: impl Write,
        args: &mut impl Iterator<Item = &'a FormatArgument>,
    ) -> Result<ControlFlow<()>, FormatError> {
        match self {
            FormatItem::Spec(spec) => spec.write(writer, args)?,
            FormatItem::Char(c) => return c.write(writer).map_err(FormatError::IoError),
        };
        Ok(ControlFlow::Continue(()))
    }
}

pub fn parse_spec_and_escape(
    fmt: &[u8],
) -> impl Iterator<Item = Result<FormatItem<EscapedChar>, FormatError>> + '_ {
    let mut current = fmt;
    std::iter::from_fn(move || match current {
        [] => return None,
        [b'%', b'%', rest @ ..] => {
            current = rest;
            Some(Ok(FormatItem::Char(EscapedChar::Byte(b'%'))))
        }
        [b'%', rest @ ..] => {
            current = rest;
            let spec = match Spec::parse(&mut current) {
                Some(spec) => spec,
                None => return Some(Err(FormatError::SpecError)),
            };
            Some(Ok(FormatItem::Spec(spec)))
        }
        [b'\\', rest @ ..] => {
            current = rest;
            Some(Ok(FormatItem::Char(parse_escape_code(&mut current))))
        }
        [c, rest @ ..] => {
            current = rest;
            Some(Ok(FormatItem::Char(EscapedChar::Byte(*c))))
        }
    })
}

fn parse_spec_only(fmt: &[u8]) -> impl Iterator<Item = Result<FormatItem<u8>, FormatError>> + '_ {
    let mut current = fmt;
    std::iter::from_fn(move || match current {
        [] => return None,
        [b'%', b'%', rest @ ..] => {
            current = rest;
            Some(Ok(FormatItem::Char(b'%')))
        }
        [b'%', rest @ ..] => {
            current = rest;
            let spec = match Spec::parse(&mut current) {
                Some(spec) => spec,
                None => return Some(Err(FormatError::SpecError)),
            };
            Some(Ok(FormatItem::Spec(spec)))
        }
        [c, rest @ ..] => {
            current = rest;
            Some(Ok(FormatItem::Char(*c)))
        }
    })
}

fn parse_escape_only(fmt: &[u8]) -> impl Iterator<Item = EscapedChar> + '_ {
    let mut current = fmt;
    std::iter::from_fn(move || match current {
        [] => return None,
        [b'\\', rest @ ..] => {
            current = rest;
            Some(parse_escape_code(&mut current))
        }
        [c, rest @ ..] => {
            current = rest;
            Some(EscapedChar::Byte(*c))
        }
    })
}

/// Write a formatted string to stdout.
///
/// `format_string` contains the template and `args` contains the
/// arguments to render into the template.
///
/// See also [`sprintf`], which creates a new formatted [`String`].
///
/// # Examples
///
/// ```rust
/// use uucore::format::{printf, FormatArgument};
///
/// printf("hello %s", &[FormatArgument::String("world".into())]).unwrap();
/// // prints "hello world"
/// ```
pub fn printf<'a>(
    format_string: impl AsRef<[u8]>,
    arguments: impl IntoIterator<Item = &'a FormatArgument>,
) -> Result<(), FormatError> {
    printf_writer(stdout(), format_string, arguments)
}

fn printf_writer<'a>(
    mut writer: impl Write,
    format_string: impl AsRef<[u8]>,
    args: impl IntoIterator<Item = &'a FormatArgument>,
) -> Result<(), FormatError> {
    let mut args = args.into_iter();
    for item in parse_spec_only(format_string.as_ref()) {
        item?.write(&mut writer, &mut args)?;
    }
    Ok(())
}

/// Create a new formatted string.
///
/// `format_string` contains the template and `args` contains the
/// arguments to render into the template.
///
/// See also [`printf`], which prints to stdout.
///
/// # Examples
///
/// ```rust
/// use uucore::format::{sprintf, FormatArgument};
///
/// let s = sprintf("hello %s", &[FormatArgument::String("world".into())]).unwrap();
/// let s = std::str::from_utf8(&s).unwrap();
/// assert_eq!(s, "hello world");
/// ```
pub fn sprintf<'a>(
    format_string: impl AsRef<[u8]>,
    arguments: impl IntoIterator<Item = &'a FormatArgument>,
) -> Result<Vec<u8>, FormatError> {
    let mut writer = Vec::new();
    printf_writer(&mut writer, format_string, arguments)?;
    Ok(writer)
}

/// A parsed format for a single float value
///
/// This is used by `seq`. It can be constructed with [`FloatFormat::parse`]
/// and can write a value with [`FloatFormat::fmt`].
///
/// It can only accept a single specification without any asterisk parameters.
/// If it does get more specifications, it will return an error.
pub struct Format<F: Formatter> {
    prefix: Vec<u8>,
    suffix: Vec<u8>,
    formatter: F,
}

impl<F: Formatter> Format<F> {
    pub fn parse(format_string: impl AsRef<[u8]>) -> Result<Self, FormatError> {
        let mut iter = parse_spec_only(format_string.as_ref());

        let mut prefix = Vec::new();
        let mut spec = None;
        for item in &mut iter {
            match item? {
                FormatItem::Spec(s) => {
                    spec = Some(s);
                    break;
                }
                FormatItem::Char(c) => prefix.push(c),
            }
        }

        let Some(spec) = spec else {
            return Err(FormatError::SpecError);
        };

        let formatter = F::try_from_spec(spec)?;

        let mut suffix = Vec::new();
        for item in &mut iter {
            match item? {
                FormatItem::Spec(_) => {
                    return Err(FormatError::SpecError);
                }
                FormatItem::Char(c) => suffix.push(c),
            }
        }

        Ok(Self {
            prefix,
            suffix,
            formatter,
        })
    }

    pub fn fmt(&self, mut w: impl Write, f: F::Input) -> std::io::Result<()> {
        w.write_all(&self.prefix)?;
        self.formatter.fmt(&mut w, f)?;
        w.write_all(&self.suffix)?;
        Ok(())
    }
}
