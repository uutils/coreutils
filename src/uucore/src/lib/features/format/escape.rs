// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Parsing of escape sequences

use crate::format::FormatError;

#[derive(Debug)]
pub enum EscapedChar {
    /// A single byte
    Byte(u8),
    /// A unicode character
    Char(char),
    /// A character prefixed with a backslash (i.e. an invalid escape sequence)
    Backslash(u8),
    /// Specifies that the string should stop (`\c`)
    End,
}

#[derive(Clone, Copy, Default)]
pub enum OctalParsing {
    #[default]
    TwoDigits = 2,
    ThreeDigits = 3,
}

#[derive(Clone, Copy)]
enum Base {
    Oct(OctalParsing),
    Hex,
}

impl Base {
    fn as_base(&self) -> u8 {
        match self {
            Self::Oct(_) => 8,
            Self::Hex => 16,
        }
    }

    fn max_digits(&self) -> u8 {
        match self {
            Self::Oct(parsing) => *parsing as u8,
            Self::Hex => 2,
        }
    }

    fn convert_digit(&self, c: u8) -> Option<u8> {
        match self {
            Self::Oct(_) => {
                if matches!(c, b'0'..=b'7') {
                    Some(c - b'0')
                } else {
                    None
                }
            }
            Self::Hex => match c {
                b'0'..=b'9' => Some(c - b'0'),
                b'A'..=b'F' => Some(c - b'A' + 10),
                b'a'..=b'f' => Some(c - b'a' + 10),
                _ => None,
            },
        }
    }
}

/// Parse the numeric part of the `\xHHH` and `\0NNN` escape sequences
fn parse_code(input: &mut &[u8], base: Base) -> Option<u8> {
    // All arithmetic on `ret` needs to be wrapping, because octal input can
    // take 3 digits, which is 9 bits, and therefore more than what fits in a
    // `u8`. GNU just seems to wrap these values.
    // Note that if we instead make `ret` a `u32` and use `char::from_u32` will
    // yield incorrect results because it will interpret values larger than
    // `u8::MAX` as unicode.
    let [c, rest @ ..] = input else { return None };
    let mut ret = base.convert_digit(*c)?;
    *input = rest;

    for _ in 1..base.max_digits() {
        let [c, rest @ ..] = input else { break };
        let Some(n) = base.convert_digit(*c) else {
            break;
        };
        ret = ret.wrapping_mul(base.as_base()).wrapping_add(n);
        *input = rest;
    }

    Some(ret)
}

// spell-checker:disable-next
/// Parse `\uHHHH` and `\UHHHHHHHH`
fn parse_unicode(input: &mut &[u8], digits: u8) -> Result<char, EscapeError> {
    if let Some((new_digits, rest)) = input.split_at_checked(digits as usize) {
        *input = rest;
        let ret = new_digits
            .iter()
            .map(|c| Base::Hex.convert_digit(*c))
            .collect::<Option<Vec<u8>>>()
            .ok_or(EscapeError::MissingHexadecimalNumber)?
            .iter()
            .map(|n| *n as u32)
            .reduce(|ret, n| ret.wrapping_mul(Base::Hex.as_base() as u32).wrapping_add(n))
            .expect("must have multiple digits in unicode string");
        char::from_u32(ret).ok_or_else(|| EscapeError::InvalidCharacters(new_digits.to_vec()))
    } else {
        Err(EscapeError::MissingHexadecimalNumber)
    }
}

/// Represents an invalid escape sequence.
#[derive(Debug, PartialEq)]
pub enum EscapeError {
    InvalidCharacters(Vec<u8>),
    MissingHexadecimalNumber,
}

/// Parse an escape sequence, like `\n` or `\xff`, etc.
pub fn parse_escape_code(
    rest: &mut &[u8],
    zero_octal_parsing: OctalParsing,
) -> Result<EscapedChar, FormatError> {
    if let [c, new_rest @ ..] = rest {
        // This is for the \NNN syntax for octal sequences.
        // Note that '0' is intentionally omitted because that
        // would be the \0NNN syntax.
        if let b'1'..=b'7' = c {
            if let Some(parsed) = parse_code(rest, Base::Oct(OctalParsing::ThreeDigits)) {
                return Ok(EscapedChar::Byte(parsed));
            }
        }

        *rest = new_rest;
        match c {
            b'\\' => Ok(EscapedChar::Byte(b'\\')),
            b'"' => Ok(EscapedChar::Byte(b'"')),
            b'a' => Ok(EscapedChar::Byte(b'\x07')),
            b'b' => Ok(EscapedChar::Byte(b'\x08')),
            b'c' => Ok(EscapedChar::End),
            b'e' => Ok(EscapedChar::Byte(b'\x1b')),
            b'f' => Ok(EscapedChar::Byte(b'\x0c')),
            b'n' => Ok(EscapedChar::Byte(b'\n')),
            b'r' => Ok(EscapedChar::Byte(b'\r')),
            b't' => Ok(EscapedChar::Byte(b'\t')),
            b'v' => Ok(EscapedChar::Byte(b'\x0b')),
            b'x' => {
                if let Some(c) = parse_code(rest, Base::Hex) {
                    Ok(EscapedChar::Byte(c))
                } else {
                    Err(FormatError::MissingHex)
                }
            }
            b'0' => Ok(EscapedChar::Byte(
                parse_code(rest, Base::Oct(zero_octal_parsing)).unwrap_or(b'\0'),
            )),
            b'u' => match parse_unicode(rest, 4) {
                Ok(c) => Ok(EscapedChar::Char(c)),
                Err(EscapeError::MissingHexadecimalNumber) => Err(FormatError::MissingHex),
                Err(EscapeError::InvalidCharacters(chars)) => {
                    Err(FormatError::InvalidCharacter('u', chars))
                }
            },
            b'U' => match parse_unicode(rest, 8) {
                Ok(c) => Ok(EscapedChar::Char(c)),
                Err(EscapeError::MissingHexadecimalNumber) => Err(FormatError::MissingHex),
                Err(EscapeError::InvalidCharacters(chars)) => {
                    Err(FormatError::InvalidCharacter('U', chars))
                }
            },
            c => Ok(EscapedChar::Backslash(*c)),
        }
    } else {
        Ok(EscapedChar::Byte(b'\\'))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod parse_unicode {
        use super::*;

        #[test]
        fn parse_ascii() {
            let input = b"2a";
            assert_eq!(parse_unicode(&mut &input[..], 2), Ok('*'));

            let input = b"002A";
            assert_eq!(parse_unicode(&mut &input[..], 4), Ok('*'));
        }

        #[test]
        fn parse_emoji_codepoint() {
            let input = b"0001F60A";
            assert_eq!(parse_unicode(&mut &input[..], 8), Ok('😊'));
        }

        #[test]
        fn no_characters() {
            let input = b"";
            assert_eq!(
                parse_unicode(&mut &input[..], 8),
                Err(EscapeError::MissingHexadecimalNumber)
            );
        }

        #[test]
        fn incomplete_hexadecimal_number() {
            let input = b"123";
            assert_eq!(
                parse_unicode(&mut &input[..], 4),
                Err(EscapeError::MissingHexadecimalNumber)
            );
        }

        #[test]
        fn invalid_hex() {
            let input = b"duck";
            assert_eq!(
                parse_unicode(&mut &input[..], 4),
                Err(EscapeError::MissingHexadecimalNumber)
            );
        }

        #[test]
        fn surrogate_code_point() {
            let input = b"d800";
            assert_eq!(
                parse_unicode(&mut &input[..], 4),
                Err(EscapeError::InvalidCharacters(Vec::from(b"d800")))
            );
        }
    }
}
