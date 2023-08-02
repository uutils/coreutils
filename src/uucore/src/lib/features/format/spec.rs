// spell-checker:ignore (vars) charf decf floatf intf scif strf Cninety

use super::{FormatArgument, FormatError};
use std::{fmt::Display, io::Write};

pub enum Spec {
    Char {
        width: Option<CanAsterisk<usize>>,
        align_left: bool,
    },
    String {
        width: Option<CanAsterisk<usize>>,
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

#[derive(Clone, Copy)]
pub enum UnsignedIntVariant {
    Decimal,
    Octal(Prefix),
    Hexadecimal(Case, Prefix),
}

#[derive(Clone, Copy)]

pub enum FloatVariant {
    Decimal,
    Scientific,
    Shortest,
    Hexadecimal,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Case {
    Lowercase,
    Uppercase,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Prefix {
    No,
    Yes,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ForceDecimal {
    No,
    Yes,
}

#[derive(Clone, Copy)]
pub enum PositiveSign {
    None,
    Plus,
    Space,
}

#[derive(Clone, Copy)]
pub enum NumberAlignment {
    Left,
    RightSpace,
    RightZero,
}

/// Precision and width specified might use an asterisk to indicate that they are
/// determined by an argument.
#[derive(Clone, Copy)]
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

        Some(match rest.get(0)? {
            b'c' => Spec::Char {
                width,
                align_left: minus,
            },
            b's' => Spec::String {
                width,
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
        mut writer: impl Write,
        mut args: impl Iterator<Item = FormatArgument>,
    ) -> Result<(), FormatError> {
        match self {
            &Spec::Char { width, align_left } => {
                let width = resolve_asterisk(width, &mut args)?.unwrap_or(0);
                let arg = next_arg(&mut args)?;
                match arg {
                    FormatArgument::Char(c) => write_padded(writer, c, width, false, align_left),
                    _ => Err(FormatError::InvalidArgument(arg)),
                }
            }
            &Spec::String { width, align_left } => {
                let width = resolve_asterisk(width, &mut args)?.unwrap_or(0);
                let arg = next_arg(&mut args)?;
                match arg {
                    FormatArgument::String(s) => write_padded(writer, s, width, false, align_left),
                    _ => Err(FormatError::InvalidArgument(arg)),
                }
            }
            &Spec::SignedInt {
                width,
                positive_sign,
                alignment,
            } => {
                let width = resolve_asterisk(width, &mut args)?.unwrap_or(0);

                let arg = next_arg(&mut args)?;
                let FormatArgument::SignedInt(i) = arg else {
                    return Err(FormatError::InvalidArgument(arg));
                };

                if i >= 0 {
                    match positive_sign {
                        PositiveSign::None => Ok(()),
                        PositiveSign::Plus => write!(writer, "+"),
                        PositiveSign::Space => write!(writer, " "),
                    }
                    .map_err(FormatError::IoError)?;
                }

                match alignment {
                    NumberAlignment::Left => write!(writer, "{i:<width$}"),
                    NumberAlignment::RightSpace => write!(writer, "{i:>width$}"),
                    NumberAlignment::RightZero => write!(writer, "{i:0>width$}"),
                }
                .map_err(FormatError::IoError)
            }
            &Spec::UnsignedInt {
                variant,
                width,
                alignment,
            } => {
                let width = resolve_asterisk(width, &mut args)?.unwrap_or(0);

                let arg = next_arg(args)?;
                let FormatArgument::SignedInt(i) = arg else {
                    return Err(FormatError::InvalidArgument(arg));
                };

                let s = match variant {
                    UnsignedIntVariant::Decimal => format!("{i}"),
                    UnsignedIntVariant::Octal(Prefix::No) => format!("{i:o}"),
                    UnsignedIntVariant::Octal(Prefix::Yes) => format!("{i:#o}"),
                    UnsignedIntVariant::Hexadecimal(Case::Lowercase, Prefix::No) => {
                        format!("{i:x}")
                    }
                    UnsignedIntVariant::Hexadecimal(Case::Lowercase, Prefix::Yes) => {
                        format!("{i:#x}")
                    }
                    UnsignedIntVariant::Hexadecimal(Case::Uppercase, Prefix::No) => {
                        format!("{i:X}")
                    }
                    UnsignedIntVariant::Hexadecimal(Case::Uppercase, Prefix::Yes) => {
                        format!("{i:#X}")
                    }
                };

                match alignment {
                    NumberAlignment::Left => write!(writer, "{s:<width$}"),
                    NumberAlignment::RightSpace => write!(writer, "{s:>width$}"),
                    NumberAlignment::RightZero => write!(writer, "{s:0>width$}"),
                }
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
                let FormatArgument::Float(f) = arg else {
                    return Err(FormatError::InvalidArgument(arg));
                };

                match positive_sign {
                    PositiveSign::None => Ok(()),
                    PositiveSign::Plus => write!(writer, "+"),
                    PositiveSign::Space => write!(writer, " "),
                }
                .map_err(FormatError::IoError)?;

                let s = match variant {
                    FloatVariant::Decimal => format_float_decimal(f, precision, case, force_decimal),
                    FloatVariant::Scientific => {
                        format_float_scientific(f, precision, case, force_decimal)
                    }
                    FloatVariant::Shortest => format_float_shortest(f, precision, case, force_decimal),
                    FloatVariant::Hexadecimal => todo!(),
                };

                match alignment {
                    NumberAlignment::Left => write!(writer, "{s:<width$}"),
                    NumberAlignment::RightSpace => write!(writer, "{s:>width$}"),
                    NumberAlignment::RightZero => write!(writer, "{s:0>width$}"),
                }
                .map_err(FormatError::IoError)
            }
        }
    }
}

fn format_float_decimal(
    f: f64,
    precision: usize,
    case: Case,
    force_decimal: ForceDecimal,
) -> String {
    if !f.is_finite() {
        let mut s = format!("{f}");
        if case == Case::Lowercase {
            s.make_ascii_uppercase();
        }
        return s;
    }

    if precision == 0 && force_decimal == ForceDecimal::Yes {
        format!("{f:.0}.")
    } else {
        format!("{f:.*}", precision)
    }
}

fn format_float_scientific(
    f: f64,
    precision: usize,
    case: Case,
    force_decimal: ForceDecimal,
) -> String {
    // If the float is NaN, -Nan, Inf or -Inf, format like any other float
    if !f.is_finite() {
        let mut s = format!("{f}");
        if case == Case::Lowercase {
            s.make_ascii_uppercase();
        }
        return s;
    }

    let exponent: i32 = f.log10().floor() as i32;
    let normalized = f / 10.0_f64.powi(exponent);

    let additional_dot = if precision == 0 && ForceDecimal::Yes == force_decimal {
        "."
    } else {
        ""
    };

    let exp_char = match case {
        Case::Lowercase => 'e',
        Case::Uppercase => 'E',
    };

    format!(
        "{normalized:.*}{additional_dot}{exp_char}{exponent:+03}",
        precision
    )
}

// TODO: This could be optimized. It's not terribly important though.
fn format_float_shortest(
    f: f64,
    precision: usize,
    case: Case,
    force_decimal: ForceDecimal,
) -> String {
    let a = format_float_decimal(f, precision, case, force_decimal);
    let b = format_float_scientific(f, precision, case, force_decimal);

    if a.len() > b.len() {
        b
    } else {
        a
    }
}

fn resolve_asterisk(
    option: Option<CanAsterisk<usize>>,
    args: impl Iterator<Item = FormatArgument>,
) -> Result<Option<usize>, FormatError> {
    Ok(match option {
        None => None,
        Some(CanAsterisk::Asterisk) => {
            let arg = next_arg(args)?;
            match arg {
                FormatArgument::UnsignedInt(u) => match usize::try_from(u) {
                    Ok(u) => Some(u),
                    Err(_) => return Err(FormatError::InvalidArgument(arg)),
                },
                _ => return Err(FormatError::InvalidArgument(arg)),
            }
        }
        Some(CanAsterisk::Fixed(w)) => Some(w),
    })
}

fn next_arg(
    mut arguments: impl Iterator<Item = FormatArgument>,
) -> Result<FormatArgument, FormatError> {
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
