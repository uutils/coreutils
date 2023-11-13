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
// spell-checker:ignore (vars) charf decf floatf intf scif strf Cninety

pub mod num_format;
mod spec;

use spec::Spec;
use std::{
    error::Error,
    fmt::Display,
    io::{stdout, Write},
};

use crate::error::UError;

use self::num_format::Formatter;

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
enum FormatItem {
    /// A format specifier
    Spec(Spec),
    /// Some plain text
    Text(Vec<u8>),
    /// A single character
    ///
    /// Added in addition to `Text` as an optimization.
    Char(u8),
}

#[derive(Clone, Debug)]
pub enum FormatArgument {
    Char(char),
    String(String),
    UnsignedInt(u64),
    SignedInt(i64),
    Float(f64),
    // Special argument that gets coerced into the other variants
    Unparsed(String),
}

impl FormatItem {
    fn write<'a>(
        &self,
        mut writer: impl Write,
        args: &mut impl Iterator<Item = &'a FormatArgument>,
    ) -> Result<(), FormatError> {
        match self {
            FormatItem::Spec(spec) => spec.write(writer, args),
            FormatItem::Text(bytes) => writer.write_all(bytes).map_err(FormatError::IoError),
            FormatItem::Char(char) => writer.write_all(&[*char]).map_err(FormatError::IoError),
        }
    }
}

fn parse_iter(fmt: &[u8]) -> impl Iterator<Item = Result<FormatItem, FormatError>> + '_ {
    let mut rest = fmt;
    std::iter::from_fn(move || {
        if rest.is_empty() {
            return None;
        }

        match rest.iter().position(|c| *c == b'%') {
            None => {
                let final_text = rest;
                rest = &[];
                Some(Ok(FormatItem::Text(final_text.into())))
            }
            Some(0) => {
                // Handle the spec
                rest = &rest[1..];
                match rest.get(0) {
                    None => Some(Ok(FormatItem::Char(b'%'))),
                    Some(b'%') => {
                        rest = &rest[1..];
                        Some(Ok(FormatItem::Char(b'%')))
                    }
                    Some(_) => {
                        let spec = match Spec::parse(&mut rest) {
                            Some(spec) => spec,
                            None => return Some(Err(dbg!(FormatError::SpecError))),
                        };
                        Some(Ok(FormatItem::Spec(spec)))
                    }
                }
            }
            Some(i) => {
                // The `after` slice includes the % so it will be handled correctly
                // in the next iteration.
                let (before, after) = rest.split_at(i);
                rest = after;
                return Some(Ok(FormatItem::Text(before.into())));
            }
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
/// use uucore::format::printf;
///
/// printf("hello %s", &["world".to_string()]).unwrap();
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
    for item in parse_iter(format_string.as_ref()) {
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
/// use uucore::format::sprintf;
///
/// let s = sprintf("hello %s", &["world".to_string()]).unwrap();
/// assert_eq!(s, "hello world".to_string());
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
        let mut iter = parse_iter(format_string.as_ref());

        let mut prefix = Vec::new();
        let mut spec = None;
        for item in &mut iter {
            match item? {
                FormatItem::Spec(s) => {
                    spec = Some(s);
                    break;
                }
                FormatItem::Text(t) => prefix.extend_from_slice(&t),
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
                    return Err(dbg!(FormatError::SpecError));
                }
                FormatItem::Text(t) => suffix.extend_from_slice(&t),
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
