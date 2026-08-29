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
use crate::{
    NonUtf8OsStrError,
    error::{UError, strip_errno},
    translate, translate_text,
};
pub use spec::Spec;
use std::{
    error::Error,
    fmt::Display,
    io::{Write, stdout},
    marker::PhantomData,
    ops::{ControlFlow, Range},
};

use os_display::Quotable;

#[derive(Debug)]
pub enum FormatError {
    /// The spec that failed to parse and its byte range in the format string.
    SpecError(Vec<u8>, Range<usize>),
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
    /// Carries its byte range in the format string, when it came from one.
    MissingHex(Option<Range<usize>>),
    /// The hexadecimal characters represent a code point that cannot represent a
    /// Unicode character (e.g., a surrogate code point)
    /// Carries its byte range in the format string, when it came from one.
    InvalidCharacter(char, Vec<u8>, Option<Range<usize>>),
    InvalidEncoding(NonUtf8OsStrError),
}

impl FormatError {
    /// Attach the byte range of the token that raised a parse-time error.
    ///
    /// Escape errors are constructed where only the escape itself is in
    /// sight; the format parser knows where that escape sat and fills the
    /// span in here. Errors parsed out of other text — a `%b` argument, an
    /// `echo` operand — keep `None`.
    fn spanned(self, span: Range<usize>) -> Self {
        match self {
            Self::MissingHex(_) => Self::MissingHex(Some(span)),
            Self::InvalidCharacter(c, digits, _) => Self::InvalidCharacter(c, digits, Some(span)),
            other => other,
        }
    }
}

/// The error for a spec [`Spec::parse`] rejected, carrying the byte range it
/// occupies in `fmt`.
///
/// The range runs from the `%` that [`Spec::parse`] had already consumed to the
/// end of what it rejected. The caller passes where that `%` is rather than
/// letting this work it out from `slice`, which would go wrong for a `slice`
/// that is not a subslice of `fmt`.
///
/// # Arguments
///
/// * `fmt` - The whole format string.
/// * `percent` - The offset in `fmt` of the `%` this spec starts with.
/// * `slice` - The subslice of `fmt` the failed parse returned.
fn spec_error(fmt: &[u8], percent: usize, slice: &[u8]) -> FormatError {
    let start = slice.as_ptr() as usize - fmt.as_ptr() as usize;
    debug_assert!(start <= fmt.len() && start + slice.len() <= fmt.len());
    debug_assert!(percent < start);
    FormatError::SpecError(slice.to_vec(), percent..start + slice.len())
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
        // Everything quoted below is text the user typed and has to come back
        // unchanged, hence translate_text! rather than translate!.
        let message = match self {
            Self::SpecError(s, _) => {
                translate_text!("format-error-invalid-spec", "spec" => String::from_utf8_lossy(s))
            }
            Self::TooManySpecs(s) => {
                translate_text!("format-error-too-many-specs", "format" => String::from_utf8_lossy(s))
            }
            Self::NeedAtLeastOneSpec(s) => {
                translate_text!("format-error-no-spec", "format" => String::from_utf8_lossy(s))
            }
            Self::EndsWithPercent(s) => {
                translate_text!("format-error-ends-with-percent", "format" => String::from_utf8_lossy(s).quote())
            }
            Self::InvalidPrecision(precision) => {
                translate_text!("format-error-invalid-precision", "precision" => precision)
            }
            // TODO: Error message below needs some work
            Self::WrongSpecType => translate!("format-error-wrong-spec-type"),
            Self::IoError(e) => translate_text!("format-error-write", "error" => strip_errno(e)),
            Self::NoMoreArguments => translate!("format-error-no-more-arguments"),
            Self::InvalidArgument(_) => translate!("format-error-invalid-argument"),
            Self::MissingHex(_) => translate!("format-error-missing-hex"),
            Self::InvalidCharacter(escape_char, digits, _) => translate_text!(
                "format-error-invalid-universal-character",
                "escape" => escape_char,
                "digits" => String::from_utf8_lossy(digits)
            ),
            Self::InvalidEncoding(no) => return no.fmt(f),
        };
        f.write_str(&message)
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

/// Reject a precision larger than printf/C allows (`i32::MAX`).
///
/// A precision near `usize::MAX` would otherwise overflow the precision/exponent
/// arithmetic in the float formatters, so we cap it the way C `printf` does.
pub(crate) fn check_precision(precision: usize) -> Result<(), FormatError> {
    if precision > i32::MAX as usize {
        Err(FormatError::InvalidPrecision(precision.to_string()))
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
            Self::Spec(spec) => spec.write(writer, args),
            Self::Char(c) => c.write(writer).map_err(FormatError::IoError),
        }
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
            let percent = fmt.len() - current.len();
            current = rest;
            let spec = match Spec::parse(&mut current) {
                Ok(spec) => spec,
                Err(slice) => return Some(Err(spec_error(fmt, percent, slice))),
            };
            Some(Ok(FormatItem::Spec(spec)))
        }
        [b'\\', rest @ ..] => {
            let start = fmt.len() - current.len();
            current = rest;
            Some(
                parse_escape_code(&mut current, OctalParsing::default())
                    .map(FormatItem::Char)
                    .map_err(|e| e.spanned(start..fmt.len() - current.len())),
            )
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
            let percent = fmt.len() - current.len();
            current = rest;
            let spec = match Spec::parse(&mut current) {
                Ok(spec) => spec,
                Err(slice) => return Some(Err(spec_error(fmt, percent, slice))),
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

        let spec =
            spec.ok_or_else(|| FormatError::NeedAtLeastOneSpec(format_string.as_ref().to_vec()))?;

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

#[cfg(test)]
mod tests {
    use super::{FormatError, check_precision};

    #[test]
    fn check_precision_caps_at_i32_max() {
        // Anything up to i32::MAX is accepted.
        assert!(check_precision(0).is_ok());
        assert!(check_precision(42).is_ok());
        assert!(check_precision(i32::MAX as usize).is_ok());

        // Anything above i32::MAX is rejected, reporting the offending value.
        let precision = i32::MAX as usize + 1;
        match check_precision(precision) {
            Err(FormatError::InvalidPrecision(reported)) => {
                assert_eq!(reported, precision.to_string());
            }
            other => panic!("expected InvalidPrecision, got {other:?}"),
        }
        assert!(matches!(
            check_precision(usize::MAX),
            Err(FormatError::InvalidPrecision(_))
        ));
    }
}
