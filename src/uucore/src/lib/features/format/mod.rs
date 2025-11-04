// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore extendedbigdecimal

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
//! of these cases is provided by [`parse_spec_only`], [`parse_escape_only`]
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
mod spec;

pub use self::escape::{EscapedChar, OctalParsing};
use crate::extendedbigdecimal::ExtendedBigDecimal;
pub use argument::{FormatArgument, FormatArguments};

use self::{escape::parse_escape_code, num_format::Formatter};
use crate::{NonUtf8OsStrError, error::UError};
pub use spec::Spec;
use std::{
    error::Error,
    fmt::Display,
    io::{Write, stdout},
    marker::PhantomData,
    ops::ControlFlow,
};

use os_display::Quotable;

#[derive(Debug)]
pub enum FormatError {
    SpecError(Vec<u8>),
    IoError(std::io::Error),
    NoMoreArguments,
    InvalidArgument(FormatArgument),
    TooManySpecs(Vec<u8>),
    NeedAtLeastOneSpec(Vec<u8>),
    WrongSpecType,
    InvalidPrecision(String),
    /// The format specifier ends with a %, as in `%f%`.
    EndsWithPercent(Vec<u8>),
    /// The escape sequence `\x` appears without a literal hexadecimal value.
    MissingHex,
    /// The hexadecimal characters represent a code point that cannot represent a
    /// Unicode character (e.g., a surrogate code point)
    InvalidCharacter(char, Vec<u8>),
    InvalidEncoding(NonUtf8OsStrError),
}

impl Error for FormatError {}
impl UError for FormatError {}

impl From<std::io::Error> for FormatError {
    fn from(value: std::io::Error) -> Self {
        Self::IoError(value)
    }
}

impl From<NonUtf8OsStrError> for FormatError {
    fn from(value: NonUtf8OsStrError) -> Self {
        Self::InvalidEncoding(value)
    }
}

impl Display for FormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SpecError(s) => write!(
                f,
                "%{}: invalid conversion specification",
                String::from_utf8_lossy(s)
            ),
            Self::TooManySpecs(s) => write!(
                f,
                "format '{}' has too many % directives",
                String::from_utf8_lossy(s)
            ),
            Self::NeedAtLeastOneSpec(s) => write!(
                f,
                "format '{}' has no % directive",
                String::from_utf8_lossy(s)
            ),
            Self::EndsWithPercent(s) => {
                write!(f, "format {} ends in %", String::from_utf8_lossy(s).quote())
            }
            Self::InvalidPrecision(precision) => write!(f, "invalid precision: '{precision}'"),
            // TODO: Error message below needs some work
            Self::WrongSpecType => write!(f, "wrong % directive type was given"),
            Self::IoError(_) => write!(f, "write error"),
            Self::NoMoreArguments => write!(f, "no more arguments"),
            Self::InvalidArgument(_) => write!(f, "invalid argument"),
            Self::MissingHex => write!(f, "missing hexadecimal number in escape"),
            Self::InvalidCharacter(escape_char, digits) => write!(
                f,
                "invalid universal character name \\{escape_char}{}",
                String::from_utf8_lossy(digits)
            ),
            Self::InvalidEncoding(no) => no.fmt(f),
        }
    }
}

/// Maximum width for formatting to prevent memory allocation panics.
/// Rust's formatter will panic when trying to allocate memory for very large widths.
/// This limit is somewhat arbitrary but should be well above any practical use case
/// while still preventing formatter panics.
const MAX_FORMAT_WIDTH: usize = 1_000_000;

/// Check if a width is too large for formatting.
/// Returns an error if the width exceeds MAX_FORMAT_WIDTH.
fn check_width(width: usize) -> std::io::Result<()> {
    if width > MAX_FORMAT_WIDTH {
        Err(std::io::Error::new(
            std::io::ErrorKind::OutOfMemory,
            "formatting width too large",
        ))
    } else {
        Ok(())
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
    pub fn write(
        &self,
        writer: impl Write,
        args: &mut FormatArguments,
    ) -> Result<ControlFlow<()>, FormatError> {
        match self {
            Self::Spec(spec) => spec.write(writer, args)?,
            Self::Char(c) => return c.write(writer).map_err(FormatError::IoError),
        }
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
            Some(parse_escape_code(&mut current, OctalParsing::default()).map(FormatItem::Char))
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
        [b'%'] => Some(Err(FormatError::EndsWithPercent(fmt.to_vec()))),
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
pub fn parse_escape_only(
    fmt: &[u8],
    zero_octal_parsing: OctalParsing,
) -> impl Iterator<Item = EscapedChar> + '_ {
    let mut current = fmt;
    std::iter::from_fn(move || match current {
        [] => None,
        [b'\\', rest @ ..] => {
            current = rest;
            Some(
                parse_escape_code(&mut current, zero_octal_parsing)
                    .unwrap_or(EscapedChar::Backslash(b'x')),
            )
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
    let args = args.into_iter().cloned().collect::<Vec<_>>();
    let mut args = FormatArguments::new(&args);
    for item in parse_spec_only(format_string.as_ref()) {
        if item?.write(&mut writer, &mut args)?.is_break() {
            break;
        }
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

/// A format for a single numerical value of type T
///
/// This is used by `seq` and `csplit`. It can be constructed with [`Format::from_formatter`]
/// or [`Format::parse`] and can write a value with [`Format::fmt`].
///
/// [`Format::parse`] can only accept a single specification without any asterisk parameters.
/// If it does get more specifications, it will return an error.
pub struct Format<F: Formatter<T>, T> {
    prefix: Vec<u8>,
    suffix: Vec<u8>,
    formatter: F,
    _marker: PhantomData<T>,
}

impl<F: Formatter<T>, T> Format<F, T> {
    pub fn from_formatter(formatter: F) -> Self {
        Self {
            prefix: Vec::<u8>::new(),
            suffix: Vec::<u8>::new(),
            formatter,
            _marker: PhantomData,
        }
    }

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
            match item {
                // If the `format_string` is of the form `%f%f` or
                // `%f%`, then return an error.
                Ok(FormatItem::Spec(_)) | Err(FormatError::EndsWithPercent(_)) => {
                    return Err(FormatError::TooManySpecs(format_string.as_ref().to_vec()));
                }
                Ok(FormatItem::Char(c)) => suffix.push(c),
                Err(e) => return Err(e),
            }
        }

        Ok(Self {
            prefix,
            suffix,
            formatter,
            _marker: PhantomData,
        })
    }

    pub fn fmt(&self, mut w: impl Write, f: T) -> std::io::Result<()> {
        w.write_all(&self.prefix)?;
        self.formatter.fmt(&mut w, f)?;
        w.write_all(&self.suffix)?;
        Ok(())
    }
}
