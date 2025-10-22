// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use super::ExtendedBigDecimal;
use crate::format::spec::ArgumentLocation;
use crate::{
    error::set_exit_code,
    os_str_as_bytes,
    parser::num_parser::{ExtendedParser, ExtendedParserError},
    quoting_style::{QuotingStyle, locale_aware_escape_name},
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

    pub fn next_char(&mut self, position: &ArgumentLocation) -> u8 {
        match self.next_arg(position) {
            Some(FormatArgument::Char(c)) => *c as u8,
            Some(FormatArgument::Unparsed(os)) => match os_str_as_bytes(os) {
                Ok(bytes) => bytes.first().copied().unwrap_or(b'\0'),
                Err(_) => b'\0',
            },
            _ => b'\0',
        }
    }

    pub fn next_string(&mut self, position: &ArgumentLocation) -> &'a OsStr {
        match self.next_arg(position) {
            Some(FormatArgument::Unparsed(os) | FormatArgument::String(os)) => os,
            _ => "".as_ref(),
        }
    }

    pub fn next_i64(&mut self, position: &ArgumentLocation) -> i64 {
        match self.next_arg(position) {
            Some(FormatArgument::SignedInt(n)) => *n,
            Some(FormatArgument::Unparsed(os)) => Self::get_num::<i64>(os),
            _ => 0,
        }
    }

    pub fn next_u64(&mut self, position: &ArgumentLocation) -> u64 {
        match self.next_arg(position) {
            Some(FormatArgument::UnsignedInt(n)) => *n,
            Some(FormatArgument::Unparsed(os)) => Self::get_num::<u64>(os),
            _ => 0,
        }
    }

    pub fn next_extended_big_decimal(&mut self, position: &ArgumentLocation) -> ExtendedBigDecimal {
        match self.next_arg(position) {
            Some(FormatArgument::Float(n)) => n.clone(),
            Some(FormatArgument::Unparsed(os)) => Self::get_num::<ExtendedBigDecimal>(os),
            _ => ExtendedBigDecimal::zero(),
        }
    }

    // Parse an OsStr that we know to start with a '/"
    fn parse_quote_start<T>(os: &OsStr) -> Result<T, ExtendedParserError<T>>
    where
        T: ExtendedParser + From<u8> + From<u32> + Default,
    {
        // If this fails (this can only happens on Windows), then just
        // return NotNumeric.
        let Ok(s) = os_str_as_bytes(os) else {
            return Err(ExtendedParserError::NotNumeric);
        };

        let (Some((b'"', bytes)) | Some((b'\'', bytes))) = s.split_first() else {
            // This really can't happen, the string we are given must start with '/".
            debug_assert!(false);
            return Err(ExtendedParserError::NotNumeric);
        };

        if bytes.is_empty() {
            return Err(ExtendedParserError::NotNumeric);
        }

        let (val, len) = if let Some(c) = bytes
            .utf8_chunks()
            .next()
            .expect("bytes should not be empty")
            .valid()
            .chars()
            .next()
        {
            // Valid UTF-8 character, cast the codepoint to u32 then T
            // (largest unicode codepoint is only 3 bytes, so this is safe)
            ((c as u32).into(), c.len_utf8())
        } else {
            // Not a valid UTF-8 character, use the first byte
            (bytes[0].into(), 1)
        };
        // Emit a warning if there are additional characters
        if bytes.len() > len {
            return Err(ExtendedParserError::PartialMatch(
                val,
                String::from_utf8_lossy(&bytes[len..]).into_owned(),
            ));
        }

        Ok(val)
    }

    fn get_num<T>(os: &OsStr) -> T
    where
        T: ExtendedParser + From<u8> + From<u32> + Default,
    {
        let s = os.to_string_lossy();
        let first = s.as_bytes().first().copied();

        let quote_start = first == Some(b'"') || first == Some(b'\'');
        let parsed = if quote_start {
            // The string begins with a quote
            Self::parse_quote_start(os)
        } else {
            T::extended_parse(&s)
        };

        // Get the best possible value, even if parsed was an error.
        extract_value(parsed, &s, quote_start)
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

fn extract_value<T: Default>(
    p: Result<T, ExtendedParserError<T>>,
    input: &str,
    quote_start: bool,
) -> T {
    match p {
        Ok(v) => v,
        Err(e) => {
            set_exit_code(1);
            let input = locale_aware_escape_name(OsStr::new(input), QuotingStyle::C_NO_QUOTES);
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
                    if quote_start {
                        set_exit_code(0);
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
        assert_eq!(b'a', args.next_char(&ArgumentLocation::NextArgument));
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
        assert_eq!(b'z', args.next_char(&ArgumentLocation::NextArgument));
        assert_eq!(b'y', args.next_char(&ArgumentLocation::NextArgument));
        args.start_next_batch();
        assert!(!args.is_exhausted());

        // Second batch - same pattern
        assert_eq!(b'x', args.next_char(&ArgumentLocation::NextArgument));
        assert_eq!(b'w', args.next_char(&ArgumentLocation::NextArgument));
        args.start_next_batch();
        assert!(!args.is_exhausted());

        // Third batch - same pattern
        assert_eq!(b'v', args.next_char(&ArgumentLocation::NextArgument));
        assert_eq!(b'u', args.next_char(&ArgumentLocation::NextArgument));
        args.start_next_batch();
        assert!(!args.is_exhausted());

        // Fourth batch - same pattern (last batch)
        assert_eq!(b't', args.next_char(&ArgumentLocation::NextArgument));
        assert_eq!(b's', args.next_char(&ArgumentLocation::NextArgument));
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
        assert_eq!(b'a', args.next_char(&ArgumentLocation::NextArgument));
        assert_eq!("hello", args.next_string(&ArgumentLocation::NextArgument));
        args.start_next_batch();
        assert!(!args.is_exhausted());

        // Second batch - same pattern
        assert_eq!(b'1', args.next_char(&ArgumentLocation::NextArgument)); // First byte of 123
        assert_eq!("world", args.next_string(&ArgumentLocation::NextArgument));
        args.start_next_batch();
        assert!(!args.is_exhausted());

        // Third batch - same pattern (last batch)
        assert_eq!(b'z', args.next_char(&ArgumentLocation::NextArgument));
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
        assert_eq!(b'b', args.next_char(&non_zero_pos(2))); // Position 2
        assert_eq!(b'a', args.next_char(&non_zero_pos(1))); // Position 1
        assert_eq!(b'c', args.next_char(&non_zero_pos(3))); // Position 3
        args.start_next_batch();
        assert!(!args.is_exhausted());

        // Second batch - same positional pattern
        assert_eq!(b'e', args.next_char(&non_zero_pos(2))); // Position 2
        assert_eq!(b'd', args.next_char(&non_zero_pos(1))); // Position 1
        assert_eq!(b'f', args.next_char(&non_zero_pos(3))); // Position 3
        args.start_next_batch();
        assert!(!args.is_exhausted());

        // Third batch - same positional pattern (last batch)
        assert_eq!(b'h', args.next_char(&non_zero_pos(2))); // Position 2
        assert_eq!(b'g', args.next_char(&non_zero_pos(1))); // Position 1
        assert_eq!(b'i', args.next_char(&non_zero_pos(3))); // Position 3
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
        assert_eq!(b'a', args.next_char(&ArgumentLocation::NextArgument)); // Sequential
        assert_eq!(b'c', args.next_char(&non_zero_pos(3))); // Positional
        args.start_next_batch();
        assert!(!args.is_exhausted());

        // Second batch - same mixed pattern
        assert_eq!(b'd', args.next_char(&ArgumentLocation::NextArgument)); // Sequential
        assert_eq!(b'f', args.next_char(&non_zero_pos(3))); // Positional
        args.start_next_batch();
        assert!(!args.is_exhausted());

        // Last batch - same mixed pattern
        assert_eq!(b'g', args.next_char(&ArgumentLocation::NextArgument)); // Sequential
        assert_eq!(b'\0', args.next_char(&non_zero_pos(3))); // Out of bounds
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
        assert_eq!(10, args.next_i64(&ArgumentLocation::NextArgument));
        assert_eq!(20, args.next_u64(&ArgumentLocation::NextArgument));
        let result = args.next_extended_big_decimal(&ArgumentLocation::NextArgument);
        assert_eq!(ExtendedBigDecimal::zero(), result);
        args.start_next_batch();
        assert!(!args.is_exhausted());

        // Second batch - same pattern
        assert_eq!(30, args.next_i64(&ArgumentLocation::NextArgument));
        assert_eq!(40, args.next_u64(&ArgumentLocation::NextArgument));
        let result = args.next_extended_big_decimal(&ArgumentLocation::NextArgument);
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
        assert_eq!(123, args.next_i64(&ArgumentLocation::NextArgument));
        args.start_next_batch();
        assert!(!args.is_exhausted());

        // Second batch - same pattern
        assert_eq!("hello", args.next_string(&ArgumentLocation::NextArgument));
        assert_eq!(456, args.next_i64(&ArgumentLocation::NextArgument));
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
        assert_eq!(b'a', args.next_char(&non_zero_pos(1)));
        assert_eq!("test", args.next_string(&non_zero_pos(2)));
        assert_eq!(42, args.next_u64(&non_zero_pos(3)));
        args.start_next_batch();
        assert!(!args.is_exhausted());

        // Second batch - same pattern
        assert_eq!(b'b', args.next_char(&non_zero_pos(1)));
        assert_eq!("more", args.next_string(&non_zero_pos(2)));
        assert_eq!(99, args.next_u64(&non_zero_pos(3)));
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
        assert_eq!(b'a', args.next_char(&ArgumentLocation::NextArgument));
        assert_eq!(b'c', args.next_char(&non_zero_pos(3)));
        args.start_next_batch();
        assert!(!args.is_exhausted());

        // Second batch (partial)
        assert_eq!(b'd', args.next_char(&ArgumentLocation::NextArgument));
        assert_eq!(b'\0', args.next_char(&non_zero_pos(3))); // Out of bounds
        args.start_next_batch();
        assert!(args.is_exhausted());
    }
}
