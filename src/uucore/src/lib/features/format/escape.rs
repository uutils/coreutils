#[derive(Debug)]
pub enum EscapedChar {
    Char(u8),
    Backslash(u8),
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

    fn to_digit(&self, c: u8) -> Option<u8> {
        match self {
            Base::Oct => {
                if matches!(c, b'0'..=b'7') {
                    Some(c - b'0')
                } else {
                    None
                }
            }
            Base::Hex => match c {
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
    let mut ret = base.to_digit(*c)?;
    *input = &rest[..];

    for _ in 1..base.max_digits() {
        let [c, rest @ ..] = input else { break };
        let Some(n) = base.to_digit(*c) else { break };
        ret = ret.wrapping_mul(base as u8).wrapping_add(n);
        *input = &rest[..];
    }

    Some(ret)
}

pub fn parse_escape_code(rest: &mut &[u8]) -> EscapedChar {
    if let [c, new_rest @ ..] = rest {
        // This is for the \NNN syntax for octal sequences.
        // Note that '0' is intentionally omitted because that
        // would be the \0NNN syntax.
        if let b'1'..=b'7' = c {
            if let Some(parsed) = parse_code(rest, Base::Oct) {
                return EscapedChar::Char(parsed);
            }
        }

        *rest = &new_rest[..];
        match c {
            b'\\' => EscapedChar::Char(b'\\'),
            b'a' => EscapedChar::Char(b'\x07'),
            b'b' => EscapedChar::Char(b'\x08'),
            b'c' => return EscapedChar::End,
            b'e' => EscapedChar::Char(b'\x1b'),
            b'f' => EscapedChar::Char(b'\x0c'),
            b'n' => EscapedChar::Char(b'\n'),
            b'r' => EscapedChar::Char(b'\r'),
            b't' => EscapedChar::Char(b'\t'),
            b'v' => EscapedChar::Char(b'\x0b'),
            b'x' => {
                if let Some(c) = parse_code(rest, Base::Hex) {
                    EscapedChar::Char(c)
                } else {
                    EscapedChar::Backslash(b'x')
                }
            }
            b'0' => EscapedChar::Char(parse_code(rest, Base::Oct).unwrap_or(b'\0')),
            c => EscapedChar::Backslash(*c),
        }
    } else {
        EscapedChar::Char(b'\\')
    }
}
