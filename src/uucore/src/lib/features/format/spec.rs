// spell-checker:ignore (vars) charf decf floatf intf scif strf Cninety

use super::{
    num_format::{
        self, Case, FloatVariant, ForceDecimal, Formatter, NumberAlignment, PositiveSign, Prefix,
        UnsignedIntVariant,
    },
    parse_escape_only, FormatArgument, FormatChar, FormatError,
};
use std::{fmt::Display, io::Write, ops::ControlFlow};

#[derive(Debug)]
pub enum Spec {
    Char {
        width: Option<CanAsterisk<usize>>,
        align_left: bool,
    },
    String {
        width: Option<CanAsterisk<usize>>,
        parse_escape: bool,
        align_left: bool,
    },
    SignedInt {
        width: Option<CanAsterisk<usize>>,
        positive_sign: PositiveSign,
        alignment: NumberAlignment,
    },
    UnsignedInt {
        variant: UnsignedIntVariant,
        width: Option<CanAsterisk<usize>>,
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

impl Spec {
    pub fn parse(rest: &mut &[u8]) -> Option<Self> {
        // Based on the C++ reference, the spec format looks like:
        //
        //   %[flags][width][.precision][length]specifier
        //
        // However, we have already parsed the '%'.

        let mut minus = false;
        let mut plus = false;
        let mut space = false;
        let mut hash = false;
        let mut zero = false;

        while let Some(x @ (b'-' | b'+' | b' ' | b'#' | b'0')) = rest.get(0) {
            match x {
                b'-' => minus = true,
                b'+' => plus = true,
                b' ' => space = true,
                b'#' => hash = true,
                b'0' => zero = true,
                _ => unreachable!(),
            }
            *rest = &rest[1..]
        }

        let width = eat_asterisk_or_number(rest);

        let precision = if let Some(b'.') = rest.get(0) {
            *rest = &rest[1..];
            Some(eat_asterisk_or_number(rest).unwrap_or(CanAsterisk::Fixed(0)))
        } else {
            None
        };

        let length = rest.get(0).and_then(|c| {
            Some(match c {
                b'h' => {
                    if let Some(b'h') = rest.get(1) {
                        *rest = &rest[1..];
                        Length::Char
                    } else {
                        Length::Short
                    }
                }
                b'l' => {
                    if let Some(b'l') = rest.get(1) {
                        *rest = &rest[1..];
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

        if length.is_some() {
            *rest = &rest[1..];
        }

        let type_spec = rest.get(0)?;
        *rest = &rest[1..];
        Some(match type_spec {
            b'c' => Spec::Char {
                width,
                align_left: minus,
            },
            b's' => Spec::String {
                width,
                parse_escape: false,
                align_left: minus,
            },
            b'b' => Spec::String {
                width,
                parse_escape: true,
                align_left: minus,
            },
            b'd' | b'i' => Spec::SignedInt {
                width,
                alignment: match (minus, zero) {
                    (true, _) => NumberAlignment::Left,
                    (false, true) => NumberAlignment::RightZero,
                    (false, false) => NumberAlignment::RightSpace,
                },
                positive_sign: match (plus, space) {
                    (true, _) => PositiveSign::Plus,
                    (false, true) => PositiveSign::Space,
                    (false, false) => PositiveSign::None,
                },
            },
            c @ (b'u' | b'o' | b'x' | b'X') => {
                let prefix = match hash {
                    false => Prefix::No,
                    true => Prefix::Yes,
                };
                let alignment = match (minus, zero) {
                    (true, _) => NumberAlignment::Left,
                    (false, true) => NumberAlignment::RightZero,
                    (false, false) => NumberAlignment::RightSpace,
                };
                let variant = match c {
                    b'u' => UnsignedIntVariant::Decimal,
                    b'o' => UnsignedIntVariant::Octal(prefix),
                    b'x' => UnsignedIntVariant::Hexadecimal(Case::Lowercase, prefix),
                    b'X' => UnsignedIntVariant::Hexadecimal(Case::Uppercase, prefix),
                    _ => unreachable!(),
                };
                Spec::UnsignedInt {
                    variant,
                    width,
                    alignment,
                }
            }
            c @ (b'f' | b'F' | b'e' | b'E' | b'g' | b'G' | b'a' | b'A') => Spec::Float {
                width,
                precision,
                variant: match c {
                    b'f' | b'F' => FloatVariant::Decimal,
                    b'e' | b'E' => FloatVariant::Scientific,
                    b'g' | b'G' => FloatVariant::Shortest,
                    b'a' | b'A' => FloatVariant::Hexadecimal,
                    _ => unreachable!(),
                },
                force_decimal: match hash {
                    false => ForceDecimal::No,
                    true => ForceDecimal::Yes,
                },
                case: match c.is_ascii_uppercase() {
                    false => Case::Lowercase,
                    true => Case::Uppercase,
                },
                alignment: match (minus, zero) {
                    (true, _) => NumberAlignment::Left,
                    (false, true) => NumberAlignment::RightZero,
                    (false, false) => NumberAlignment::RightSpace,
                },
                positive_sign: match (plus, space) {
                    (true, _) => PositiveSign::Plus,
                    (false, true) => PositiveSign::Space,
                    (false, false) => PositiveSign::None,
                },
            },
            _ => return None,
        })
    }

    pub fn write<'a>(
        &self,
        writer: impl Write,
        mut args: impl Iterator<Item = &'a FormatArgument>,
    ) -> Result<(), FormatError> {
        match self {
            &Spec::Char { width, align_left } => {
                let width = resolve_asterisk(width, &mut args)?.unwrap_or(0);
                let arg = next_arg(&mut args)?;
                match arg.get_char() {
                    Some(c) => write_padded(writer, c, width, false, align_left),
                    _ => Err(FormatError::InvalidArgument(arg.clone())),
                }
            }
            &Spec::String {
                width,
                parse_escape,
                align_left,
            } => {
                let width = resolve_asterisk(width, &mut args)?.unwrap_or(0);
                let arg = next_arg(&mut args)?;
                let Some(s) = arg.get_str() else {
                    return Err(FormatError::InvalidArgument(arg.clone()));
                };
                if parse_escape {
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
                    write_padded(
                        writer,
                        std::str::from_utf8(&parsed).expect("TODO: Accept invalid utf8"),
                        width,
                        false,
                        align_left,
                    )
                } else {
                    write_padded(writer, s, width, false, align_left)
                }
            }
            &Spec::SignedInt {
                width,
                positive_sign,
                alignment,
            } => {
                let width = resolve_asterisk(width, &mut args)?.unwrap_or(0);

                let arg = next_arg(&mut args)?;
                let Some(i) = arg.get_i64() else {
                    return Err(FormatError::InvalidArgument(arg.clone()));
                };

                num_format::SignedInt {
                    width,
                    positive_sign,
                    alignment,
                }
                .fmt(writer, i)
                .map_err(FormatError::IoError)
            }
            &Spec::UnsignedInt {
                variant,
                width,
                alignment,
            } => {
                let width = resolve_asterisk(width, &mut args)?.unwrap_or(0);

                let arg = next_arg(args)?;
                let Some(i) = arg.get_u64() else {
                    return Err(FormatError::InvalidArgument(arg.clone()));
                };

                num_format::UnsignedInt {
                    variant,
                    width,
                    alignment,
                }
                .fmt(writer, i)
                .map_err(FormatError::IoError)
            }
            &Spec::Float {
                variant,
                case,
                force_decimal,
                width,
                positive_sign,
                alignment,
                precision,
            } => {
                let width = resolve_asterisk(width, &mut args)?.unwrap_or(0);
                let precision = resolve_asterisk(precision, &mut args)?.unwrap_or(6);

                let arg = next_arg(args)?;
                let Some(f) = arg.get_f64() else {
                    return Err(FormatError::InvalidArgument(arg.clone()));
                };

                num_format::Float {
                    variant,
                    case,
                    force_decimal,
                    width,
                    positive_sign,
                    alignment,
                    precision,
                }
                .fmt(writer, f)
                .map_err(FormatError::IoError)
            }
        }
    }
}

fn resolve_asterisk<'a>(
    option: Option<CanAsterisk<usize>>,
    args: impl Iterator<Item = &'a FormatArgument>,
) -> Result<Option<usize>, FormatError> {
    Ok(match option {
        None => None,
        Some(CanAsterisk::Asterisk) => {
            let arg = next_arg(args)?;
            match arg.get_u64() {
                Some(u) => match usize::try_from(u) {
                    Ok(u) => Some(u),
                    Err(_) => return Err(FormatError::InvalidArgument(arg.clone())),
                },
                _ => return Err(FormatError::InvalidArgument(arg.clone())),
            }
        }
        Some(CanAsterisk::Fixed(w)) => Some(w),
    })
}

fn next_arg<'a>(
    mut arguments: impl Iterator<Item = &'a FormatArgument>,
) -> Result<&'a FormatArgument, FormatError> {
    arguments.next().ok_or(FormatError::NoMoreArguments)
}

fn write_padded(
    mut writer: impl Write,
    text: impl Display,
    width: usize,
    pad_zero: bool,
    left: bool,
) -> Result<(), FormatError> {
    match (left, pad_zero) {
        (false, false) => write!(writer, "{text: >width$}"),
        (false, true) => write!(writer, "{text:0>width$}"),
        // 0 is ignored if we pad left.
        (true, _) => write!(writer, "{text: <width$}"),
    }
    .map_err(FormatError::IoError)
}

fn eat_asterisk_or_number(rest: &mut &[u8]) -> Option<CanAsterisk<usize>> {
    if let Some(b'*') = rest.get(0) {
        *rest = &rest[1..];
        Some(CanAsterisk::Asterisk)
    } else {
        eat_number(rest).map(CanAsterisk::Fixed)
    }
}

fn eat_number(rest: &mut &[u8]) -> Option<usize> {
    match rest.iter().position(|b| !b.is_ascii_digit()) {
        None | Some(0) => None,
        Some(i) => {
            // TODO: This might need to handle errors better
            // For example in case of overflow.
            let parsed = std::str::from_utf8(&rest[..i]).unwrap().parse().unwrap();
            *rest = &rest[i..];
            Some(parsed)
        }
    }
}
