// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use super::{ExtendedBigDecimal, FormatError};
use crate::format::spec::ArgumentLocation;
use crate::{
    error::set_exit_code,
    os_str_as_bytes_verbose, os_str_as_str_verbose,
    parser::num_parser::{ExtendedParser, ExtendedParserError},
    quoting_style::{Quotes, QuotingStyle, escape_name},
    show_error, show_warning,
};
use os_display::Quotable;
use std::{
    ffi::{OsStr, OsString},
    num::NonZero,
};

/// An argument for formatting
///
/// Each of these variants is only accepted by their respective directives. For
/// example, [`FormatArgument::Char`] requires a `%c` directive.
///
/// The [`FormatArgument::Unparsed`] variant contains a string that can be
/// parsed into other types. This is used by the `printf` utility.
#[derive(Clone, Debug, PartialEq)]
pub enum FormatArgument {
    Char(char),
    String(OsString),
    UnsignedInt(u64),
    SignedInt(i64),
    Float(ExtendedBigDecimal),
    /// Special argument that gets coerced into the other variants
    Unparsed(OsString),
}

/// A struct that holds a slice of format arguments and provides methods to access them
#[derive(Debug, PartialEq)]
pub struct FormatArguments<'a> {
    args: &'a [FormatArgument],
    next_arg_position: usize,
    highest_arg_position: Option<usize>,
    current_offset: usize,
}

impl<'a> FormatArguments<'a> {
    /// Create a new FormatArguments from a slice of FormatArgument
    pub fn new(args: &'a [FormatArgument]) -> Self {
        Self {
            args,
            next_arg_position: 0,
            highest_arg_position: None,
            current_offset: 0,
        }
    }

    /// Get the next argument that would be used
    pub fn peek_arg(&self) -> Option<&'a FormatArgument> {
        self.args.get(self.next_arg_position)
    }

    /// Check if all arguments have been consumed
    pub fn is_exhausted(&self) -> bool {
        self.current_offset >= self.args.len()
    }

    pub fn start_next_batch(&mut self) {
        self.current_offset = self
            .next_arg_position
            .max(self.highest_arg_position.map_or(0, |x| x.saturating_add(1)));
        self.next_arg_position = self.current_offset;
    }

    pub fn next_char(&mut self, position: &ArgumentLocation) -> Result<u8, FormatError> {
        match self.next_arg(position) {
            Some(FormatArgument::Char(c)) => Ok(*c as u8),
            Some(FormatArgument::Unparsed(os)) => match os_str_as_bytes_verbose(os)?.first() {
                Some(&byte) => Ok(byte),
                None => Ok(b'\0'),
            },
            _ => Ok(b'\0'),
        }
    }

    pub fn next_string(&mut self, position: &ArgumentLocation) -> &'a OsStr {
        match self.next_arg(position) {
            Some(FormatArgument::Unparsed(os) | FormatArgument::String(os)) => os,
            _ => "".as_ref(),
        }
    }

    pub fn next_i64(&mut self, position: &ArgumentLocation) -> Result<i64, FormatError> {
        match self.next_arg(position) {
            Some(FormatArgument::SignedInt(n)) => Ok(*n),
            Some(FormatArgument::Unparsed(os)) => {
                let str = os_str_as_str_verbose(os)?;

                Ok(extract_value(i64::extended_parse(str), str))
            }
            _ => Ok(0),
        }
    }

    pub fn next_u64(&mut self, position: &ArgumentLocation) -> Result<u64, FormatError> {
        match self.next_arg(position) {
            Some(FormatArgument::UnsignedInt(n)) => Ok(*n),
            Some(FormatArgument::Unparsed(os)) => {
                let s = os_str_as_str_verbose(os)?;
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
                        return Ok(first_char as u64); // Use only the first character
                    }
                    return Ok(0); // Empty quotes
                }
                Ok(extract_value(u64::extended_parse(s), s))
            }
            _ => Ok(0),
        }
    }

    pub fn next_extended_big_decimal(
        &mut self,
        position: &ArgumentLocation,
    ) -> Result<ExtendedBigDecimal, FormatError> {
        match self.next_arg(position) {
            Some(FormatArgument::Float(n)) => Ok(n.clone()),
            Some(FormatArgument::Unparsed(os)) => {
                let s = os_str_as_str_verbose(os)?;

                Ok(extract_value(ExtendedBigDecimal::extended_parse(s), s))
            }
            _ => Ok(ExtendedBigDecimal::zero()),
        }
    }

    fn get_at_relative_position(&mut self, pos: NonZero<usize>) -> Option<&'a FormatArgument> {
        let pos: usize = pos.into();
        let pos = (pos - 1).saturating_add(self.current_offset);
        self.highest_arg_position = Some(self.highest_arg_position.map_or(pos, |x| x.max(pos)));
        self.args.get(pos)
    }

    fn next_arg(&mut self, position: &ArgumentLocation) -> Option<&'a FormatArgument> {
        match position {
            ArgumentLocation::NextArgument => {
                let arg = self.args.get(self.next_arg_position);
                self.next_arg_position += 1;
                arg
            }
            ArgumentLocation::Position(pos) => self.get_at_relative_position(*pos),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_arguments_empty() {
        let args = FormatArguments::new(&[]);
        assert_eq!(None, args.peek_arg());
        assert!(args.is_exhausted());
    }

    #[test]
    fn test_format_arguments_single_element() {
        let mut args = FormatArguments::new(&[FormatArgument::Char('a')]);
        assert!(!args.is_exhausted());
        assert_eq!(Some(&FormatArgument::Char('a')), args.peek_arg());
        assert!(!args.is_exhausted()); // Peek shouldn't consume
        assert_eq!(
            b'a',
            args.next_char(&ArgumentLocation::NextArgument).unwrap()
        );
        args.start_next_batch();
        assert!(args.is_exhausted()); // After batch, exhausted with a single arg
        assert_eq!(None, args.peek_arg());
    }

    #[test]
    fn test_sequential_next_char() {
        // Test with consistent sequential next_char calls
        let mut args = FormatArguments::new(&[
            FormatArgument::Char('z'),
            FormatArgument::Char('y'),
            FormatArgument::Char('x'),
            FormatArgument::Char('w'),
            FormatArgument::Char('v'),
            FormatArgument::Char('u'),
            FormatArgument::Char('t'),
            FormatArgument::Char('s'),
        ]);

        // First batch - two sequential calls
        assert_eq!(
            b'z',
            args.next_char(&ArgumentLocation::NextArgument).unwrap()
        );
        assert_eq!(
            b'y',
            args.next_char(&ArgumentLocation::NextArgument).unwrap()
        );
        args.start_next_batch();
        assert!(!args.is_exhausted());

        // Second batch - same pattern
        assert_eq!(
            b'x',
            args.next_char(&ArgumentLocation::NextArgument).unwrap()
        );
        assert_eq!(
            b'w',
            args.next_char(&ArgumentLocation::NextArgument).unwrap()
        );
        args.start_next_batch();
        assert!(!args.is_exhausted());

        // Third batch - same pattern
        assert_eq!(
            b'v',
            args.next_char(&ArgumentLocation::NextArgument).unwrap()
        );
        assert_eq!(
            b'u',
            args.next_char(&ArgumentLocation::NextArgument).unwrap()
        );
        args.start_next_batch();
        assert!(!args.is_exhausted());

        // Fourth batch - same pattern (last batch)
        assert_eq!(
            b't',
            args.next_char(&ArgumentLocation::NextArgument).unwrap()
        );
        assert_eq!(
            b's',
            args.next_char(&ArgumentLocation::NextArgument).unwrap()
        );
        args.start_next_batch();
        assert!(args.is_exhausted());
    }

    #[test]
    fn test_sequential_different_methods() {
        // Test with different method types in sequence
        let args = [
            FormatArgument::Char('a'),
            FormatArgument::String("hello".into()),
            FormatArgument::Unparsed("123".into()),
            FormatArgument::String("world".into()),
            FormatArgument::Char('z'),
            FormatArgument::String("test".into()),
        ];
        let mut args = FormatArguments::new(&args);

        // First batch - next_char followed by next_string
        assert_eq!(
            b'a',
            args.next_char(&ArgumentLocation::NextArgument).unwrap()
        );
        assert_eq!("hello", args.next_string(&ArgumentLocation::NextArgument));
        args.start_next_batch();
        assert!(!args.is_exhausted());

        // Second batch - same pattern
        assert_eq!(
            b'1',
            args.next_char(&ArgumentLocation::NextArgument).unwrap()
        ); // First byte of 123
        assert_eq!("world", args.next_string(&ArgumentLocation::NextArgument));
        args.start_next_batch();
        assert!(!args.is_exhausted());

        // Third batch - same pattern (last batch)
        assert_eq!(
            b'z',
            args.next_char(&ArgumentLocation::NextArgument).unwrap()
        );
        assert_eq!("test", args.next_string(&ArgumentLocation::NextArgument));
        args.start_next_batch();
        assert!(args.is_exhausted());
    }

    fn non_zero_pos(n: usize) -> ArgumentLocation {
        ArgumentLocation::Position(NonZero::new(n).unwrap())
    }

    #[test]
    fn test_position_access_pattern() {
        // Test with consistent positional access patterns
        let mut args = FormatArguments::new(&[
            FormatArgument::Char('a'),
            FormatArgument::Char('b'),
            FormatArgument::Char('c'),
            FormatArgument::Char('d'),
            FormatArgument::Char('e'),
            FormatArgument::Char('f'),
            FormatArgument::Char('g'),
            FormatArgument::Char('h'),
            FormatArgument::Char('i'),
        ]);

        // First batch - positional access
        assert_eq!(b'b', args.next_char(&non_zero_pos(2)).unwrap()); // Position 2
        assert_eq!(b'a', args.next_char(&non_zero_pos(1)).unwrap()); // Position 1
        assert_eq!(b'c', args.next_char(&non_zero_pos(3)).unwrap()); // Position 3
        args.start_next_batch();
        assert!(!args.is_exhausted());

        // Second batch - same positional pattern
        assert_eq!(b'e', args.next_char(&non_zero_pos(2)).unwrap()); // Position 2
        assert_eq!(b'd', args.next_char(&non_zero_pos(1)).unwrap()); // Position 1
        assert_eq!(b'f', args.next_char(&non_zero_pos(3)).unwrap()); // Position 3
        args.start_next_batch();
        assert!(!args.is_exhausted());

        // Third batch - same positional pattern (last batch)
        assert_eq!(b'h', args.next_char(&non_zero_pos(2)).unwrap()); // Position 2
        assert_eq!(b'g', args.next_char(&non_zero_pos(1)).unwrap()); // Position 1
        assert_eq!(b'i', args.next_char(&non_zero_pos(3)).unwrap()); // Position 3
        args.start_next_batch();
        assert!(args.is_exhausted());
    }

    #[test]
    fn test_mixed_access_pattern() {
        // Test with mixed sequential and positional access
        let mut args = FormatArguments::new(&[
            FormatArgument::Char('a'),
            FormatArgument::Char('b'),
            FormatArgument::Char('c'),
            FormatArgument::Char('d'),
            FormatArgument::Char('e'),
            FormatArgument::Char('f'),
            FormatArgument::Char('g'),
            FormatArgument::Char('h'),
        ]);

        // First batch - mix of sequential and positional
        assert_eq!(
            b'a',
            args.next_char(&ArgumentLocation::NextArgument).unwrap()
        ); // Sequential
        assert_eq!(b'c', args.next_char(&non_zero_pos(3)).unwrap()); // Positional
        args.start_next_batch();
        assert!(!args.is_exhausted());

        // Second batch - same mixed pattern
        assert_eq!(
            b'd',
            args.next_char(&ArgumentLocation::NextArgument).unwrap()
        ); // Sequential
        assert_eq!(b'f', args.next_char(&non_zero_pos(3)).unwrap()); // Positional
        args.start_next_batch();
        assert!(!args.is_exhausted());

        // Last batch - same mixed pattern
        assert_eq!(
            b'g',
            args.next_char(&ArgumentLocation::NextArgument).unwrap()
        ); // Sequential
        assert_eq!(b'\0', args.next_char(&non_zero_pos(3)).unwrap()); // Out of bounds
        args.start_next_batch();
        assert!(args.is_exhausted());
    }

    #[test]
    fn test_numeric_argument_types() {
        // Test with numeric argument types
        let args = [
            FormatArgument::SignedInt(10),
            FormatArgument::UnsignedInt(20),
            FormatArgument::Float(ExtendedBigDecimal::zero()),
            FormatArgument::SignedInt(30),
            FormatArgument::UnsignedInt(40),
            FormatArgument::Float(ExtendedBigDecimal::zero()),
        ];
        let mut args = FormatArguments::new(&args);

        // First batch - i64, u64, decimal
        assert_eq!(10, args.next_i64(&ArgumentLocation::NextArgument).unwrap());
        assert_eq!(20, args.next_u64(&ArgumentLocation::NextArgument).unwrap());
        let result = args
            .next_extended_big_decimal(&ArgumentLocation::NextArgument)
            .unwrap();
        assert_eq!(ExtendedBigDecimal::zero(), result);
        args.start_next_batch();
        assert!(!args.is_exhausted());

        // Second batch - same pattern
        assert_eq!(30, args.next_i64(&ArgumentLocation::NextArgument).unwrap());
        assert_eq!(40, args.next_u64(&ArgumentLocation::NextArgument).unwrap());
        let result = args
            .next_extended_big_decimal(&ArgumentLocation::NextArgument)
            .unwrap();
        assert_eq!(ExtendedBigDecimal::zero(), result);
        args.start_next_batch();
        assert!(args.is_exhausted());
    }

    #[test]
    fn test_unparsed_arguments() {
        // Test with unparsed arguments that get coerced
        let args = [
            FormatArgument::Unparsed("hello".into()),
            FormatArgument::Unparsed("123".into()),
            FormatArgument::Unparsed("hello".into()),
            FormatArgument::Unparsed("456".into()),
        ];
        let mut args = FormatArguments::new(&args);

        // First batch - string, number
        assert_eq!("hello", args.next_string(&ArgumentLocation::NextArgument));
        assert_eq!(123, args.next_i64(&ArgumentLocation::NextArgument).unwrap());
        args.start_next_batch();
        assert!(!args.is_exhausted());

        // Second batch - same pattern
        assert_eq!("hello", args.next_string(&ArgumentLocation::NextArgument));
        assert_eq!(456, args.next_i64(&ArgumentLocation::NextArgument).unwrap());
        args.start_next_batch();
        assert!(args.is_exhausted());
    }

    #[test]
    fn test_mixed_types_with_positions() {
        // Test with mixed types and positional access
        let args = [
            FormatArgument::Char('a'),
            FormatArgument::String("test".into()),
            FormatArgument::UnsignedInt(42),
            FormatArgument::Char('b'),
            FormatArgument::String("more".into()),
            FormatArgument::UnsignedInt(99),
        ];
        let mut args = FormatArguments::new(&args);

        // First batch - positional access of different types
        assert_eq!(b'a', args.next_char(&non_zero_pos(1)).unwrap());
        assert_eq!("test", args.next_string(&non_zero_pos(2)));
        assert_eq!(42, args.next_u64(&non_zero_pos(3)).unwrap());
        args.start_next_batch();
        assert!(!args.is_exhausted());

        // Second batch - same pattern
        assert_eq!(b'b', args.next_char(&non_zero_pos(1)).unwrap());
        assert_eq!("more", args.next_string(&non_zero_pos(2)));
        assert_eq!(99, args.next_u64(&non_zero_pos(3)).unwrap());
        args.start_next_batch();
        assert!(args.is_exhausted());
    }

    #[test]
    fn test_partial_last_batch() {
        // Test with a partial last batch (fewer elements than batch size)
        let mut args = FormatArguments::new(&[
            FormatArgument::Char('a'),
            FormatArgument::Char('b'),
            FormatArgument::Char('c'),
            FormatArgument::Char('d'),
            FormatArgument::Char('e'), // Last batch has fewer elements
        ]);

        // First batch
        assert_eq!(
            b'a',
            args.next_char(&ArgumentLocation::NextArgument).unwrap()
        );
        assert_eq!(b'c', args.next_char(&non_zero_pos(3)).unwrap());
        args.start_next_batch();
        assert!(!args.is_exhausted());

        // Second batch (partial)
        assert_eq!(
            b'd',
            args.next_char(&ArgumentLocation::NextArgument).unwrap()
        );
        assert_eq!(b'\0', args.next_char(&non_zero_pos(3)).unwrap()); // Out of bounds
        args.start_next_batch();
        assert!(args.is_exhausted());
    }
}
