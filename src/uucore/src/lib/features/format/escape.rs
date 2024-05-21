// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Parsing of escape sequences

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

#[repr(u8)]
#[derive(Clone, Copy)]
enum Base {
    Oct = 8,
    Hex = 16,
}

impl Base {
    fn max_digits(&self) -> u8 {
        match self {
            Self::Oct => 3,
            Self::Hex => 2,
        }
    }

    fn convert_digit(&self, c: u8) -> Option<u8> {
        match self {
            Self::Oct => {
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
        ret = ret.wrapping_mul(base as u8).wrapping_add(n);
        *input = rest;
    }

    Some(ret)
}

// spell-checker:disable-next
/// Parse `\uHHHH` and `\UHHHHHHHH`
// TODO: This should print warnings and possibly halt execution when it fails to parse
// TODO: If the character cannot be converted to u32, the input should be printed.
fn parse_unicode(input: &mut &[u8], digits: u8) -> Option<char> {
    let (c, rest) = input.split_first()?;
    let mut ret = Base::Hex.convert_digit(*c)? as u32;
    *input = rest;

    for _ in 1..digits {
        let (c, rest) = input.split_first()?;
        let n = Base::Hex.convert_digit(*c)?;
        ret = ret.wrapping_mul(Base::Hex as u32).wrapping_add(n as u32);
        *input = rest;
    }

    char::from_u32(ret)
}

pub fn parse_escape_code(rest: &mut &[u8]) -> EscapedChar {
    if let [c, new_rest @ ..] = rest {
        // This is for the \NNN syntax for octal sequences.
        // Note that '0' is intentionally omitted because that
        // would be the \0NNN syntax.
        if let b'1'..=b'7' = c {
            if let Some(parsed) = parse_code(rest, Base::Oct) {
                return EscapedChar::Byte(parsed);
            }
        }

        *rest = new_rest;
        match c {
            b'\\' => EscapedChar::Byte(b'\\'),
            b'"' => EscapedChar::Byte(b'"'),
            b'a' => EscapedChar::Byte(b'\x07'),
            b'b' => EscapedChar::Byte(b'\x08'),
            b'c' => EscapedChar::End,
            b'e' => EscapedChar::Byte(b'\x1b'),
            b'f' => EscapedChar::Byte(b'\x0c'),
            b'n' => EscapedChar::Byte(b'\n'),
            b'r' => EscapedChar::Byte(b'\r'),
            b't' => EscapedChar::Byte(b'\t'),
            b'v' => EscapedChar::Byte(b'\x0b'),
            b'x' => {
                if let Some(c) = parse_code(rest, Base::Hex) {
                    EscapedChar::Byte(c)
                } else {
                    EscapedChar::Backslash(b'x')
                }
            }
            b'0' => EscapedChar::Byte(parse_code(rest, Base::Oct).unwrap_or(b'\0')),
            b'u' => EscapedChar::Char(parse_unicode(rest, 4).unwrap_or('\0')),
            b'U' => EscapedChar::Char(parse_unicode(rest, 8).unwrap_or('\0')),
            c => EscapedChar::Backslash(*c),
        }
    } else {
        EscapedChar::Byte(b'\\')
    }
}
