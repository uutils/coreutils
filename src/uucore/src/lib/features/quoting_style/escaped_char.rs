// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::char::from_digit;

use super::Quotes;

// PR#6559 : Remove `]{}` from special shell chars.
const SPECIAL_SHELL_CHARS: &str = "`$&*()|[;\\'\"<>?! ";

// This implementation is heavily inspired by the std::char::EscapeDefault implementation
// in the Rust standard library. This custom implementation is needed because the
// characters \a, \b, \e, \f & \v are not recognized by Rust.
pub struct EscapedChar {
    pub state: EscapeState,
}

pub enum EscapeState {
    Done,
    Char(char),
    Backslash(char),
    ForceQuote(char),
    Octal(EscapeOctal),
}

/// Bytes we need to present as escaped octal, in the form of `\nnn` per byte.
/// Supports characters up to 4 bytes long in UTF-8.
pub struct EscapeOctal {
    bytes: [u8; 4],
    num_bytes: usize,
    byte_idx: usize,
    digit_idx: u8,
    state: EscapeOctalState,
}

enum EscapeOctalState {
    Done,
    Backslash,
    Value,
}

fn byte_to_octal_digit(byte: u8, idx: u8) -> u8 {
    (byte >> (idx * 3)) & 0o7
}

impl Iterator for EscapeOctal {
    type Item = char;

    fn next(&mut self) -> Option<char> {
        match self.state {
            EscapeOctalState::Done => None,
            EscapeOctalState::Backslash => {
                self.state = EscapeOctalState::Value;
                Some('\\')
            }
            EscapeOctalState::Value => {
                let octal_digit = byte_to_octal_digit(self.bytes[self.byte_idx], self.digit_idx);
                if self.digit_idx == 0 {
                    // Move to next byte
                    self.byte_idx += 1;
                    if self.byte_idx >= self.num_bytes {
                        self.state = EscapeOctalState::Done;
                    } else {
                        self.state = EscapeOctalState::Backslash;
                        self.digit_idx = 2;
                    }
                } else {
                    self.digit_idx -= 1;
                }
                Some(from_digit(octal_digit.into(), 8).unwrap())
            }
        }
    }
}

impl EscapeOctal {
    fn from_char(c: char) -> Self {
        if c.len_utf8() == 1 {
            return Self::from_byte(c as u8);
        }

        let mut bytes = [0; 4];
        let len = c.encode_utf8(&mut bytes).len();
        Self {
            bytes,
            num_bytes: len,
            byte_idx: 0,
            digit_idx: 2,
            state: EscapeOctalState::Backslash,
        }
    }

    fn from_byte(b: u8) -> Self {
        Self {
            bytes: [b, 0, 0, 0],
            num_bytes: 1,
            byte_idx: 0,
            digit_idx: 2,
            state: EscapeOctalState::Backslash,
        }
    }
}

impl EscapedChar {
    pub fn new_literal(c: char) -> Self {
        Self {
            state: EscapeState::Char(c),
        }
    }

    pub fn new_octal(b: u8) -> Self {
        Self {
            state: EscapeState::Octal(EscapeOctal::from_byte(b)),
        }
    }

    pub fn new_c(c: char, quotes: Quotes, dirname: bool) -> Self {
        use EscapeState::*;
        let init_state = match c {
            '\x07' => Backslash('a'),
            '\x08' => Backslash('b'),
            '\t' => Backslash('t'),
            '\n' => Backslash('n'),
            '\x0B' => Backslash('v'),
            '\x0C' => Backslash('f'),
            '\r' => Backslash('r'),
            '\\' => Backslash('\\'),
            '\'' => match quotes {
                Quotes::Single => Backslash('\''),
                _ => Char('\''),
            },
            '"' => match quotes {
                Quotes::Double => Backslash('"'),
                _ => Char('"'),
            },
            ' ' if !dirname => match quotes {
                Quotes::None => Backslash(' '),
                _ => Char(' '),
            },
            ':' if dirname => Backslash(':'),
            _ if c.is_control() => Octal(EscapeOctal::from_char(c)),
            _ => Char(c),
        };
        Self { state: init_state }
    }

    pub fn new_shell(c: char, escape: bool, quotes: Quotes) -> Self {
        use EscapeState::*;
        let init_state = match c {
            _ if !escape && c.is_control() => Char(c),
            '\x07' => Backslash('a'),
            '\x08' => Backslash('b'),
            '\t' => Backslash('t'),
            '\n' => Backslash('n'),
            '\x0B' => Backslash('v'),
            '\x0C' => Backslash('f'),
            '\r' => Backslash('r'),
            '\'' => match quotes {
                Quotes::Single => Backslash('\''),
                _ => Char('\''),
            },
            _ if c.is_control() => Octal(EscapeOctal::from_char(c)),
            _ if SPECIAL_SHELL_CHARS.contains(c) => ForceQuote(c),
            _ => Char(c),
        };
        Self { state: init_state }
    }

    pub fn hide_control(self) -> Self {
        match self.state {
            EscapeState::Char(c) if c.is_control() => Self {
                state: EscapeState::Char('?'),
            },
            _ => self,
        }
    }
}

impl Iterator for EscapedChar {
    type Item = char;

    fn next(&mut self) -> Option<char> {
        match self.state {
            EscapeState::Backslash(c) => {
                self.state = EscapeState::Char(c);
                Some('\\')
            }
            EscapeState::Char(c) | EscapeState::ForceQuote(c) => {
                self.state = EscapeState::Done;
                Some(c)
            }
            EscapeState::Done => None,
            EscapeState::Octal(ref mut iter) => iter.next(),
        }
    }
}
