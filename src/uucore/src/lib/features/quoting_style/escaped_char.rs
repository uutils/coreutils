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
/// Only supports characters up to 2 bytes long in UTF-8.
pub struct EscapeOctal {
    c: [u8; 2],
    state: EscapeOctalState,
    idx: u8,
}

enum EscapeOctalState {
    Done,
    FirstBackslash,
    FirstValue,
    LastBackslash,
    LastValue,
}

fn byte_to_octal_digit(byte: u8, idx: u8) -> u8 {
    (byte >> (idx * 3)) & 0o7
}

impl Iterator for EscapeOctal {
    type Item = char;

    fn next(&mut self) -> Option<char> {
        match self.state {
            EscapeOctalState::Done => None,
            EscapeOctalState::FirstBackslash => {
                self.state = EscapeOctalState::FirstValue;
                Some('\\')
            }
            EscapeOctalState::LastBackslash => {
                self.state = EscapeOctalState::LastValue;
                Some('\\')
            }
            EscapeOctalState::FirstValue => {
                let octal_digit = byte_to_octal_digit(self.c[0], self.idx);
                if self.idx == 0 {
                    self.state = EscapeOctalState::LastBackslash;
                    self.idx = 2;
                } else {
                    self.idx -= 1;
                }
                Some(from_digit(octal_digit.into(), 8).unwrap())
            }
            EscapeOctalState::LastValue => {
                let octal_digit = byte_to_octal_digit(self.c[1], self.idx);
                if self.idx == 0 {
                    self.state = EscapeOctalState::Done;
                } else {
                    self.idx -= 1;
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

        let mut buf = [0; 2];
        let _s = c.encode_utf8(&mut buf);
        Self {
            c: buf,
            idx: 2,
            state: EscapeOctalState::FirstBackslash,
        }
    }

    fn from_byte(b: u8) -> Self {
        Self {
            c: [0, b],
            idx: 2,
            state: EscapeOctalState::LastBackslash,
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
