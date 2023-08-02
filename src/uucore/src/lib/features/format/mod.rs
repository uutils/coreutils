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

// mod num_format;
mod spec;

use spec::Spec;
use std::io::{stdout, Write};

pub enum FormatError {
    SpecError,
    IoError(std::io::Error),
    NoMoreArguments,
    InvalidArgument(FormatArgument),
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

pub enum FormatArgument {
    Char(char),
    String(String),
    UnsignedInt(u64),
    SignedInt(i64),
    Float(f64),
}

impl FormatItem {
    fn write<'a>(&self, mut writer: impl Write, args: &mut impl Iterator<Item = FormatArgument>) -> Result<(), FormatError> {
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
                            None => return Some(Err(FormatError::SpecError)),
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
pub fn printf(format_string: &[u8], arguments: impl IntoIterator<Item = FormatArgument>) -> Result<(), FormatError> {
    printf_writer(stdout(), format_string, arguments)
}

fn printf_writer(mut writer: impl Write, format_string: &[u8], args: impl IntoIterator<Item = FormatArgument>) -> Result<(), FormatError> {
    let mut args = args.into_iter();
    for item in parse_iter(format_string) {
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
pub fn sprintf(format_string: &[u8], arguments: impl IntoIterator<Item = FormatArgument>) -> Result<Vec<u8>, FormatError> {
    let mut writer = Vec::new();
    printf_writer(&mut writer, format_string, arguments)?;
    Ok(writer)
}
