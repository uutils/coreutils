// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (vars) intmax ptrdiff padlen

use crate::quoting_style::{escape_name, QuotingStyle};

use super::{
    num_format::{
        self, Case, FloatVariant, ForceDecimal, Formatter, NumberAlignment, PositiveSign, Prefix,
        UnsignedIntVariant,
    },
    parse_escape_only, ArgumentIter, FormatChar, FormatError,
};
use std::{io::Write, ops::ControlFlow};

/// A parsed specification for formatting a value
///
/// This might require more than one argument to resolve width or precision
/// values that are given as `*`.
#[derive(Debug)]
pub enum Spec {
    Char {
        width: Option<CanAsterisk<usize>>,
        align_left: bool,
    },
    String {
        precision: Option<CanAsterisk<usize>>,
        width: Option<CanAsterisk<usize>>,
        align_left: bool,
    },
    EscapedString,
    QuotedString,
    SignedInt {
        width: Option<CanAsterisk<usize>>,
        precision: Option<CanAsterisk<usize>>,
        positive_sign: PositiveSign,
        alignment: NumberAlignment,
    },
    UnsignedInt {
        variant: UnsignedIntVariant,
        width: Option<CanAsterisk<usize>>,
        precision: Option<CanAsterisk<usize>>,
        alignment: NumberAlignment,
    },
    Float {
        variant: FloatVariant,
        case: Case,
        force_decimal: ForceDecimal,
        width: Option<CanAsterisk<usize>>,
        positive_sign: PositiveSign,
        alignment: NumberAlignment,
        precision: Option<CanAsterisk<usize>>,
    },
}

/// Precision and width specified might use an asterisk to indicate that they are
/// determined by an argument.
#[derive(Clone, Copy, Debug)]
pub enum CanAsterisk<T> {
    Fixed(T),
    Asterisk,
}

/// Size of the expected type (ignored)
///
/// We ignore this parameter entirely, but we do parse it.
/// It could be used in the future if the need arises.
enum Length {
    /// signed/unsigned char ("hh")
    Char,
    /// signed/unsigned short int ("h")
    Short,
    /// signed/unsigned long int ("l")
    Long,
    /// signed/unsigned long long int ("ll")
    LongLong,
    /// intmax_t ("j")
    IntMaxT,
    /// size_t ("z")
    SizeT,
    /// ptrdiff_t ("t")
    PtfDiffT,
    /// long double ("L")
    LongDouble,
}

#[derive(Default, PartialEq, Eq)]
struct Flags {
    minus: bool,
    plus: bool,
    space: bool,
    hash: bool,
    zero: bool,
}

impl Flags {
    pub fn parse(rest: &mut &[u8], index: &mut usize) -> Self {
        let mut flags = Self::default();

        while let Some(x) = rest.get(*index) {
            match x {
                b'-' => flags.minus = true,
                b'+' => flags.plus = true,
                b' ' => flags.space = true,
                b'#' => flags.hash = true,
                b'0' => flags.zero = true,
                _ => break,
            }
            *index += 1;
        }

        flags
    }

    /// Whether any of the flags is set to true
    fn any(&self) -> bool {
        self != &Self::default()
    }
}

impl Spec {
    pub fn parse<'a>(rest: &mut &'a [u8]) -> Result<Self, &'a [u8]> {
        // Based on the C++ reference, the spec format looks like:
        //
        //   %[flags][width][.precision][length]specifier
        //
        // However, we have already parsed the '%'.
        let mut index = 0;
        let start = *rest;

        let flags = Flags::parse(rest, &mut index);

        let positive_sign = match flags {
            Flags { plus: true, .. } => PositiveSign::Plus,
            Flags { space: true, .. } => PositiveSign::Space,
            _ => PositiveSign::None,
        };

        let width = eat_asterisk_or_number(rest, &mut index);

        let precision = if let Some(b'.') = rest.get(index) {
            index += 1;
            Some(eat_asterisk_or_number(rest, &mut index).unwrap_or(CanAsterisk::Fixed(0)))
        } else {
            None
        };

        // The `0` flag is ignored if `-` is given or a precision is specified.
        // So the only case for RightZero, is when `-` is not given and the
        // precision is none.
        let alignment = if flags.minus {
            NumberAlignment::Left
        } else if flags.zero && precision.is_none() {
            NumberAlignment::RightZero
        } else {
            NumberAlignment::RightSpace
        };

        // We ignore the length. It's not really relevant to printf
        let _ = Self::parse_length(rest, &mut index);

        let Some(type_spec) = rest.get(index) else {
            return Err(&start[..index]);
        };
        index += 1;
        *rest = &start[index..];

        Ok(match type_spec {
            // GNU accepts minus, plus and space even though they are not used
            b'c' => {
                if flags.zero || flags.hash || precision.is_some() {
                    return Err(&start[..index]);
                }
                Self::Char {
                    width,
                    align_left: flags.minus,
                }
            }
            b's' => {
                if flags.zero || flags.hash {
                    return Err(&start[..index]);
                }
                Self::String {
                    precision,
                    width,
                    align_left: flags.minus,
                }
            }
            b'b' => {
                if flags.any() || width.is_some() || precision.is_some() {
                    return Err(&start[..index]);
                }
                Self::EscapedString
            }
            b'q' => {
                if flags.any() || width.is_some() || precision.is_some() {
                    return Err(&start[..index]);
                }
                Self::QuotedString
            }
            b'd' | b'i' => {
                if flags.hash {
                    return Err(&start[..index]);
                }
                Self::SignedInt {
                    width,
                    precision,
                    alignment,
                    positive_sign,
                }
            }
            c @ (b'u' | b'o' | b'x' | b'X') => {
                // Normal unsigned integer cannot have a prefix
                if *c == b'u' && flags.hash {
                    return Err(&start[..index]);
                }
                let prefix = if flags.hash { Prefix::Yes } else { Prefix::No };
                let variant = match c {
                    b'u' => UnsignedIntVariant::Decimal,
                    b'o' => UnsignedIntVariant::Octal(prefix),
                    b'x' => UnsignedIntVariant::Hexadecimal(Case::Lowercase, prefix),
                    b'X' => UnsignedIntVariant::Hexadecimal(Case::Uppercase, prefix),
                    _ => unreachable!(),
                };
                Self::UnsignedInt {
                    variant,
                    precision,
                    width,
                    alignment,
                }
            }
            c @ (b'f' | b'F' | b'e' | b'E' | b'g' | b'G' | b'a' | b'A') => Self::Float {
                width,
                precision,
                variant: match c {
                    b'f' | b'F' => FloatVariant::Decimal,
                    b'e' | b'E' => FloatVariant::Scientific,
                    b'g' | b'G' => FloatVariant::Shortest,
                    b'a' | b'A' => FloatVariant::Hexadecimal,
                    _ => unreachable!(),
                },
                force_decimal: if flags.hash {
                    ForceDecimal::Yes
                } else {
                    ForceDecimal::No
                },
                case: if c.is_ascii_uppercase() {
                    Case::Uppercase
                } else {
                    Case::Lowercase
                },
                alignment: if flags.zero && !flags.minus {
                    NumberAlignment::RightZero // float should always try to zero pad despite the precision
                } else {
                    alignment
                },
                positive_sign,
            },
            _ => return Err(&start[..index]),
        })
    }

    fn parse_length(rest: &mut &[u8], index: &mut usize) -> Option<Length> {
        // Parse 0..N length options, keep the last one
        // Even though it is just ignored. We might want to use it later and we
        // should parse those characters.
        //
        // TODO: This needs to be configurable: `seq` accepts only one length
        //       param
        let mut length = None;
        loop {
            let new_length = rest.get(*index).and_then(|c| {
                Some(match c {
                    b'h' => {
                        if let Some(b'h') = rest.get(*index + 1) {
                            *index += 1;
                            Length::Char
                        } else {
                            Length::Short
                        }
                    }
                    b'l' => {
                        if let Some(b'l') = rest.get(*index + 1) {
                            *index += 1;
                            Length::Long
                        } else {
                            Length::LongLong
                        }
                    }
                    b'j' => Length::IntMaxT,
                    b'z' => Length::SizeT,
                    b't' => Length::PtfDiffT,
                    b'L' => Length::LongDouble,
                    _ => return None,
                })
            });
            if new_length.is_some() {
                *index += 1;
                length = new_length;
            } else {
                break;
            }
        }
        length
    }

    pub fn write<'a>(
        &self,
        mut writer: impl Write,
        mut args: impl ArgumentIter<'a>,
    ) -> Result<(), FormatError> {
        match self {
            Self::Char { width, align_left } => {
                let width = resolve_asterisk(*width, &mut args)?.unwrap_or(0);
                write_padded(writer, &[args.get_char()], width, *align_left)
            }
            Self::String {
                width,
                align_left,
                precision,
            } => {
                let width = resolve_asterisk(*width, &mut args)?.unwrap_or(0);

                // GNU does do this truncation on a byte level, see for instance:
                //     printf "%.1s" 🙃
                //     > �
                // For now, we let printf panic when we truncate within a code point.
                // TODO: We need to not use Rust's formatting for aligning the output,
                // so that we can just write bytes to stdout without panicking.
                let precision = resolve_asterisk(*precision, &mut args)?;
                let s = args.get_str();
                let truncated = match precision {
                    Some(p) if p < s.len() => &s[..p],
                    _ => s,
                };
                write_padded(writer, truncated.as_bytes(), width, *align_left)
            }
            Self::EscapedString => {
                let s = args.get_str();
                let mut parsed = Vec::new();
                for c in parse_escape_only(s.as_bytes()) {
                    match c.write(&mut parsed)? {
                        ControlFlow::Continue(()) => {}
                        ControlFlow::Break(()) => {
                            // TODO: This should break the _entire execution_ of printf
                            break;
                        }
                    };
                }
                writer.write_all(&parsed).map_err(FormatError::IoError)
            }
            Self::QuotedString => {
                let s = args.get_str();
                writer
                    .write_all(
                        escape_name(
                            s.as_ref(),
                            &QuotingStyle::Shell {
                                escape: true,
                                always_quote: false,
                                show_control: false,
                            },
                        )
                        .as_bytes(),
                    )
                    .map_err(FormatError::IoError)
            }
            Self::SignedInt {
                width,
                precision,
                positive_sign,
                alignment,
            } => {
                let width = resolve_asterisk(*width, &mut args)?.unwrap_or(0);
                let precision = resolve_asterisk(*precision, &mut args)?.unwrap_or(0);
                let i = args.get_i64();

                if precision as u64 > i32::MAX as u64 {
                    return Err(FormatError::InvalidPrecision(precision.to_string()));
                }

                num_format::SignedInt {
                    width,
                    precision,
                    positive_sign: *positive_sign,
                    alignment: *alignment,
                }
                .fmt(writer, i)
                .map_err(FormatError::IoError)
            }
            Self::UnsignedInt {
                variant,
                width,
                precision,
                alignment,
            } => {
                let width = resolve_asterisk(*width, &mut args)?.unwrap_or(0);
                let precision = resolve_asterisk(*precision, &mut args)?.unwrap_or(0);
                let i = args.get_u64();

                if precision as u64 > i32::MAX as u64 {
                    return Err(FormatError::InvalidPrecision(precision.to_string()));
                }

                num_format::UnsignedInt {
                    variant: *variant,
                    precision,
                    width,
                    alignment: *alignment,
                }
                .fmt(writer, i)
                .map_err(FormatError::IoError)
            }
            Self::Float {
                variant,
                case,
                force_decimal,
                width,
                positive_sign,
                alignment,
                precision,
            } => {
                let width = resolve_asterisk(*width, &mut args)?.unwrap_or(0);
                let precision = resolve_asterisk(*precision, &mut args)?.unwrap_or(6);
                let f = args.get_f64();

                if precision as u64 > i32::MAX as u64 {
                    return Err(FormatError::InvalidPrecision(precision.to_string()));
                }

                num_format::Float {
                    width,
                    precision,
                    variant: *variant,
                    case: *case,
                    force_decimal: *force_decimal,
                    positive_sign: *positive_sign,
                    alignment: *alignment,
                }
                .fmt(writer, f)
                .map_err(FormatError::IoError)
            }
        }
    }
}

fn resolve_asterisk<'a>(
    option: Option<CanAsterisk<usize>>,
    mut args: impl ArgumentIter<'a>,
) -> Result<Option<usize>, FormatError> {
    Ok(match option {
        None => None,
        Some(CanAsterisk::Asterisk) => Some(usize::try_from(args.get_u64()).ok().unwrap_or(0)),
        Some(CanAsterisk::Fixed(w)) => Some(w),
    })
}

fn write_padded(
    mut writer: impl Write,
    text: &[u8],
    width: usize,
    left: bool,
) -> Result<(), FormatError> {
    let padlen = width.saturating_sub(text.len());
    if left {
        writer.write_all(text)?;
        write!(writer, "{: <padlen$}", "")
    } else {
        write!(writer, "{: >padlen$}", "")?;
        writer.write_all(text)
    }
    .map_err(FormatError::IoError)
}

fn eat_asterisk_or_number(rest: &mut &[u8], index: &mut usize) -> Option<CanAsterisk<usize>> {
    if let Some(b'*') = rest.get(*index) {
        *index += 1;
        Some(CanAsterisk::Asterisk)
    } else {
        eat_number(rest, index).map(CanAsterisk::Fixed)
    }
}

fn eat_number(rest: &mut &[u8], index: &mut usize) -> Option<usize> {
    match rest[*index..].iter().position(|b| !b.is_ascii_digit()) {
        None | Some(0) => None,
        Some(i) => {
            // TODO: This might need to handle errors better
            // For example in case of overflow.
            let parsed = std::str::from_utf8(&rest[*index..(*index + i)])
                .unwrap()
                .parse()
                .unwrap();
            *index += i;
            Some(parsed)
        }
    }
}
