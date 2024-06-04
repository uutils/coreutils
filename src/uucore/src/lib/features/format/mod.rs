// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! `printf`-style formatting
//!
//! Rust has excellent formatting capabilities, but the coreutils require very
//! specific formatting that needs to work exactly like the GNU utilities.
//! Naturally, the GNU behavior is based on the C `printf` functionality.
//!
//! Additionally, we need support for escape sequences for the `printf` utility.
//!
//! The [`printf`] and [`sprintf`] functions closely match the behavior of the
//! corresponding C functions: the former renders a formatted string
//! to stdout, the latter renders to a new [`String`] object.
//!
//! There are three kinds of parsing that we might want to do:
//!
//!  1. Parse only `printf` directives (for e.g. `seq`, `dd`)
//!  2. Parse only escape sequences (for e.g. `echo`)
//!  3. Parse both `printf` specifiers and escape sequences (for e.g. `printf`)
//!
//! This module aims to combine all three use cases. An iterator parsing each
//! of these cases is provided by [`parse_escape_only`], [`parse_spec_only`]
//! and [`parse_spec_and_escape`], respectively.
//!
//! There is a special [`Format`] type, which can be used to parse a format
//! string containing exactly one directive and does not use any `*` in that
//! directive. This format can be printed in a type-safe manner without failing
//! (modulo IO errors).

mod argument;
mod escape;
pub mod human;
pub mod num_format;
pub mod num_parser;
mod spec;

pub use argument::*;
use quick_error::quick_error;
use spec::Spec;
use std::{
    io,
    io::{stdout, Write},
    ops::ControlFlow,
};

use crate::error::UError;

use self::{
    escape::{parse_escape_code, EscapedChar},
    num_format::Formatter,
};

quick_error! {
    #[derive(Debug)]
    pub enum FormatError {
        SpecError(s: Vec<u8>) {
            display("%{}: invalid conversion specification", String::from_utf8_lossy(s))
        }
        IoError(err: io::Error) {
            from()
            display("io error: {}", err)
        }
        NoMoreArguments {
            display("no more arguments")
        }
        InvalidArgument(arg: FormatArgument) {
            display("invalid argument: {:?}", arg)
        }
        TooManySpecs(s: Vec<u8>) {
            display("format '{}' has too many % directives", String::from_utf8_lossy(s))
        }
        NeedAtLeastOneSpec(s: Vec<u8>) {
            display("format '{}' has no % directive", String::from_utf8_lossy(s))
        }
        WrongSpecType {
            display("wrong % directive type was given")
        }
    }
}

impl UError for FormatError {}

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
        writer.write_all(&[*self])?;
        Ok(ControlFlow::Continue(()))
    }
}

impl FormatChar for EscapedChar {
    fn write(&self, mut writer: impl Write) -> std::io::Result<ControlFlow<()>> {
        match self {
            Self::Byte(c) => {
                writer.write_all(&[*c])?;
            }
            Self::Char(c) => {
                write!(writer, "{c}")?;
            }
            Self::Backslash(c) => {
                writer.write_all(&[b'\\', *c])?;
            }
            Self::End => return Ok(ControlFlow::Break(())),
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
            Self::Spec(spec) => spec.write(writer, args)?,
            Self::Char(c) => return c.write(writer).map_err(FormatError::IoError),
        };
        Ok(ControlFlow::Continue(()))
    }
}

/// Parse a format string containing % directives and escape sequences
pub fn parse_spec_and_escape(
    fmt: &[u8],
) -> impl Iterator<Item = Result<FormatItem<EscapedChar>, FormatError>> + '_ {
    let mut current = fmt;
    std::iter::from_fn(move || match current {
        [] => None,
        [b'%', b'%', rest @ ..] => {
            current = rest;
            Some(Ok(FormatItem::Char(EscapedChar::Byte(b'%'))))
        }
        [b'%', rest @ ..] => {
            current = rest;
            let spec = match Spec::parse(&mut current) {
                Ok(spec) => spec,
                Err(slice) => return Some(Err(FormatError::SpecError(slice.to_vec()))),
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

/// Parse a format string containing % directives
pub fn parse_spec_only(
    fmt: &[u8],
) -> impl Iterator<Item = Result<FormatItem<u8>, FormatError>> + '_ {
    let mut current = fmt;
    std::iter::from_fn(move || match current {
        [] => None,
        [b'%', b'%', rest @ ..] => {
            current = rest;
            Some(Ok(FormatItem::Char(b'%')))
        }
        [b'%', rest @ ..] => {
            current = rest;
            let spec = match Spec::parse(&mut current) {
                Ok(spec) => spec,
                Err(slice) => return Some(Err(FormatError::SpecError(slice.to_vec()))),
            };
            Some(Ok(FormatItem::Spec(spec)))
        }
        [c, rest @ ..] => {
            current = rest;
            Some(Ok(FormatItem::Char(*c)))
        }
    })
}

/// Parse a format string containing escape sequences
pub fn parse_escape_only(fmt: &[u8]) -> impl Iterator<Item = EscapedChar> + '_ {
    let mut current = fmt;
    std::iter::from_fn(move || match current {
        [] => None,
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
/// This is used by `seq`. It can be constructed with [`Format::parse`]
/// and can write a value with [`Format::fmt`].
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
            return Err(FormatError::NeedAtLeastOneSpec(
                format_string.as_ref().to_vec(),
            ));
        };

        let formatter = F::try_from_spec(spec)?;

        let mut suffix = Vec::new();
        for item in &mut iter {
            match item? {
                FormatItem::Spec(_) => {
                    return Err(FormatError::TooManySpecs(format_string.as_ref().to_vec()));
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
