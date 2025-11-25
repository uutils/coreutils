// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (vars) intmax ptrdiff padlen

use super::{
    ExtendedBigDecimal, FormatChar, FormatError, OctalParsing,
    num_format::{
        self, Case, FloatVariant, ForceDecimal, Formatter, NumberAlignment, PositiveSign, Prefix,
        UnsignedIntVariant,
    },
    parse_escape_only,
};
use crate::{
    format::FormatArguments,
    os_str_as_bytes,
    quoting_style::{QuotingStyle, locale_aware_escape_name},
};
use std::{io::Write, num::NonZero, ops::ControlFlow};

/// A parsed specification for formatting a value
///
/// This might require more than one argument to resolve width or precision
/// values that are given as `*`.
#[derive(Debug)]
pub enum Spec {
    Char {
        position: ArgumentLocation,
        width: Option<CanAsterisk<usize>>,
        align_left: bool,
    },
    String {
        position: ArgumentLocation,
        precision: Option<CanAsterisk<usize>>,
        width: Option<CanAsterisk<usize>>,
        align_left: bool,
    },
    EscapedString {
        position: ArgumentLocation,
    },
    QuotedString {
        position: ArgumentLocation,
    },
    SignedInt {
        position: ArgumentLocation,
        width: Option<CanAsterisk<usize>>,
        precision: Option<CanAsterisk<usize>>,
        positive_sign: PositiveSign,
        alignment: NumberAlignment,
    },
    UnsignedInt {
        position: ArgumentLocation,
        variant: UnsignedIntVariant,
        width: Option<CanAsterisk<usize>>,
        precision: Option<CanAsterisk<usize>>,
        alignment: NumberAlignment,
    },
    Float {
        position: ArgumentLocation,
        variant: FloatVariant,
        case: Case,
        force_decimal: ForceDecimal,
        width: Option<CanAsterisk<usize>>,
        positive_sign: PositiveSign,
        alignment: NumberAlignment,
        precision: Option<CanAsterisk<usize>>,
    },
}

#[derive(Clone, Copy, Debug)]
pub enum ArgumentLocation {
    NextArgument,
    Position(NonZero<usize>),
}

/// Precision and width specified might use an asterisk to indicate that they are
/// determined by an argument.
#[derive(Clone, Copy, Debug)]
pub enum CanAsterisk<T> {
    Fixed(T),
    Asterisk(ArgumentLocation),
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
    quote: bool,
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
                b'\'' => {
                    // the thousands separator is printed with numbers using the ' flag, but
                    // this is a no-op in the "C" locale. We only save this flag for reporting errors
                    flags.quote = true;
                }
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
        // Based on the C++ reference and the Single UNIX Specification,
        // the spec format looks like:
        //
        //   %[argumentNum$][flags][width][.precision][length]specifier
        //
        // However, we have already parsed the '%'.
        let mut index = 0;
        let start = *rest;

        // Check for a positional specifier (%m$)
        let Some(position) = eat_argument_position(rest, &mut index) else {
            return Err(&start[..index]);
        };

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
                    position,
                    width,
                    align_left: flags.minus,
                }
            }
            b's' => {
                if flags.zero || flags.hash || flags.quote {
                    return Err(&start[..index]);
                }
                Self::String {
                    position,
                    precision,
                    width,
                    align_left: flags.minus,
                }
            }
            b'b' => {
                if flags.any() || width.is_some() || precision.is_some() {
                    return Err(&start[..index]);
                }
                Self::EscapedString { position }
            }
            b'q' => {
                if flags.any() || width.is_some() || precision.is_some() {
                    return Err(&start[..index]);
                }
                Self::QuotedString { position }
            }
            b'd' | b'i' => {
                if flags.hash {
                    return Err(&start[..index]);
                }
                Self::SignedInt {
                    position,
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
                    position,
                    variant,
                    precision,
                    width,
                    alignment,
                }
            }
            c @ (b'f' | b'F' | b'e' | b'E' | b'g' | b'G' | b'a' | b'A') => Self::Float {
                position,
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

    pub fn write(
        &self,
        mut writer: impl Write,
        args: &mut FormatArguments,
    ) -> Result<(), FormatError> {
        match self {
            Self::Char {
                width,
                align_left,
                position,
            } => {
                let (width, neg_width) = resolve_asterisk_width(*width, args).unwrap_or_default();
                write_padded(
                    writer,
                    &[args.next_char(position)],
                    width,
                    *align_left || neg_width,
                )
            }
            Self::String {
                width,
                align_left,
                precision,
                position,
            } => {
                let (width, neg_width) = resolve_asterisk_width(*width, args).unwrap_or_default();

                // GNU does do this truncation on a byte level, see for instance:
                //     printf "%.1s" 🙃
                //     > �
                // For now, we let printf panic when we truncate within a code point.
                // TODO: We need to not use Rust's formatting for aligning the output,
                // so that we can just write bytes to stdout without panicking.
                let precision = resolve_asterisk_precision(*precision, args);
                let os_str = args.next_string(position);
                let bytes = os_str_as_bytes(os_str)?;

                let truncated = match precision {
                    Some(p) if p < os_str.len() => &bytes[..p],
                    _ => bytes,
                };
                write_padded(writer, truncated, width, *align_left || neg_width)
            }
            Self::EscapedString { position } => {
                let os_str = args.next_string(position);
                let bytes = os_str_as_bytes(os_str)?;
                let mut parsed = Vec::<u8>::new();

                for c in parse_escape_only(bytes, OctalParsing::ThreeDigits) {
                    match c.write(&mut parsed)? {
                        ControlFlow::Continue(()) => {}
                        ControlFlow::Break(()) => {
                            // TODO: This should break the _entire execution_ of printf
                            break;
                        }
                    }
                }
                writer.write_all(&parsed).map_err(FormatError::IoError)
            }
            Self::QuotedString { position } => {
                let s = locale_aware_escape_name(
                    args.next_string(position),
                    QuotingStyle::SHELL_ESCAPE,
                );
                let bytes = os_str_as_bytes(&s)?;
                writer.write_all(bytes).map_err(FormatError::IoError)
            }
            Self::SignedInt {
                width,
                precision,
                positive_sign,
                alignment,
                position,
            } => {
                let (width, neg_width) = resolve_asterisk_width(*width, args).unwrap_or((0, false));
                let precision = resolve_asterisk_precision(*precision, args).unwrap_or_default();
                let i = args.next_i64(position);

                if precision as u64 > i32::MAX as u64 {
                    return Err(FormatError::InvalidPrecision(precision.to_string()));
                }

                num_format::SignedInt {
                    width,
                    precision,
                    positive_sign: *positive_sign,
                    alignment: if neg_width {
                        NumberAlignment::Left
                    } else {
                        *alignment
                    },
                }
                .fmt(writer, i)
                .map_err(FormatError::IoError)
            }
            Self::UnsignedInt {
                variant,
                width,
                precision,
                alignment,
                position,
            } => {
                let (width, neg_width) = resolve_asterisk_width(*width, args).unwrap_or((0, false));
                let precision = resolve_asterisk_precision(*precision, args).unwrap_or_default();
                let i = args.next_u64(position);

                if precision as u64 > i32::MAX as u64 {
                    return Err(FormatError::InvalidPrecision(precision.to_string()));
                }

                num_format::UnsignedInt {
                    variant: *variant,
                    precision,
                    width,
                    alignment: if neg_width {
                        NumberAlignment::Left
                    } else {
                        *alignment
                    },
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
                position,
            } => {
                let (width, neg_width) = resolve_asterisk_width(*width, args).unwrap_or((0, false));
                let precision = resolve_asterisk_precision(*precision, args);
                let f: ExtendedBigDecimal = args.next_extended_big_decimal(position);

                if precision.is_some_and(|p| p as u64 > i32::MAX as u64) {
                    return Err(FormatError::InvalidPrecision(
                        precision.unwrap().to_string(),
                    ));
                }

                num_format::Float {
                    width,
                    precision,
                    variant: *variant,
                    case: *case,
                    force_decimal: *force_decimal,
                    positive_sign: *positive_sign,
                    alignment: if neg_width {
                        NumberAlignment::Left
                    } else {
                        *alignment
                    },
                }
                .fmt(writer, &f)
                .map_err(FormatError::IoError)
            }
        }
    }
}

/// Determine the width, potentially getting a value from args
/// Returns the non-negative width and whether the value should be left-aligned.
fn resolve_asterisk_width(
    option: Option<CanAsterisk<usize>>,
    args: &mut FormatArguments,
) -> Option<(usize, bool)> {
    match option {
        None => None,
        Some(CanAsterisk::Asterisk(loc)) => {
            let nb = args.next_i64(&loc);
            if nb < 0 {
                Some((usize::try_from(-(nb as isize)).ok().unwrap_or(0), true))
            } else {
                Some((usize::try_from(nb).ok().unwrap_or(0), false))
            }
        }
        Some(CanAsterisk::Fixed(w)) => Some((w, false)),
    }
}

/// Determines the precision, which should (if defined)
/// be a non-negative number.
fn resolve_asterisk_precision(
    option: Option<CanAsterisk<usize>>,
    args: &mut FormatArguments,
) -> Option<usize> {
    match option {
        None => None,
        Some(CanAsterisk::Asterisk(loc)) => match args.next_i64(&loc) {
            v if v >= 0 => usize::try_from(v).ok(),
            v if v < 0 => Some(0usize),
            _ => None,
        },
        Some(CanAsterisk::Fixed(w)) => Some(w),
    }
}

fn write_padded(
    mut writer: impl Write,
    text: &[u8],
    width: usize,
    left: bool,
) -> Result<(), FormatError> {
    let padlen = width.saturating_sub(text.len());

    // Check if the padding length is too large for formatting
    super::check_width(padlen).map_err(FormatError::IoError)?;

    if left {
        writer.write_all(text)?;
        write!(writer, "{: <padlen$}", "")
    } else {
        write!(writer, "{: >padlen$}", "")?;
        writer.write_all(text)
    }
    .map_err(FormatError::IoError)
}

/// Check for a number ending with a '$'
fn eat_argument_position(rest: &mut &[u8], index: &mut usize) -> Option<ArgumentLocation> {
    let original_index = *index;
    if let Some(pos) = eat_number(rest, index) {
        if let Some(&b'$') = rest.get(*index) {
            *index += 1;
            Some(ArgumentLocation::Position(NonZero::new(pos)?))
        } else {
            *index = original_index;
            Some(ArgumentLocation::NextArgument)
        }
    } else {
        *index = original_index;
        Some(ArgumentLocation::NextArgument)
    }
}

fn eat_asterisk_or_number(rest: &mut &[u8], index: &mut usize) -> Option<CanAsterisk<usize>> {
    if let Some(b'*') = rest.get(*index) {
        *index += 1;
        // Check for a positional specifier (*m$)
        Some(CanAsterisk::Asterisk(eat_argument_position(rest, index)?))
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

#[cfg(test)]
mod tests {
    use super::*;

    mod resolve_asterisk_width {
        use super::*;
        use crate::format::FormatArgument;

        #[test]
        fn no_width() {
            assert_eq!(
                None,
                resolve_asterisk_width(None, &mut FormatArguments::new(&[]))
            );
        }

        #[test]
        fn fixed_width() {
            assert_eq!(
                Some((42, false)),
                resolve_asterisk_width(
                    Some(CanAsterisk::Fixed(42)),
                    &mut FormatArguments::new(&[])
                )
            );
        }

        #[test]
        fn asterisks_with_numbers() {
            assert_eq!(
                Some((42, false)),
                resolve_asterisk_width(
                    Some(CanAsterisk::Asterisk(ArgumentLocation::NextArgument)),
                    &mut FormatArguments::new(&[FormatArgument::SignedInt(42)]),
                )
            );
            assert_eq!(
                Some((42, false)),
                resolve_asterisk_width(
                    Some(CanAsterisk::Asterisk(ArgumentLocation::NextArgument)),
                    &mut FormatArguments::new(&[FormatArgument::Unparsed("42".into())]),
                )
            );

            assert_eq!(
                Some((42, true)),
                resolve_asterisk_width(
                    Some(CanAsterisk::Asterisk(ArgumentLocation::NextArgument)),
                    &mut FormatArguments::new(&[FormatArgument::SignedInt(-42)]),
                )
            );
            assert_eq!(
                Some((42, true)),
                resolve_asterisk_width(
                    Some(CanAsterisk::Asterisk(ArgumentLocation::NextArgument)),
                    &mut FormatArguments::new(&[FormatArgument::Unparsed("-42".into())]),
                )
            );

            assert_eq!(
                Some((2, false)),
                resolve_asterisk_width(
                    Some(CanAsterisk::Asterisk(ArgumentLocation::Position(
                        NonZero::new(2).unwrap()
                    ))),
                    &mut FormatArguments::new(&[
                        FormatArgument::Unparsed("1".into()),
                        FormatArgument::Unparsed("2".into()),
                        FormatArgument::Unparsed("3".into())
                    ]),
                )
            );
        }
    }

    mod resolve_asterisk_precision {
        use super::*;
        use crate::format::FormatArgument;

        #[test]
        fn no_width() {
            assert_eq!(
                None,
                resolve_asterisk_precision(None, &mut FormatArguments::new(&[]))
            );
        }

        #[test]
        fn fixed_width() {
            assert_eq!(
                Some(42),
                resolve_asterisk_precision(
                    Some(CanAsterisk::Fixed(42)),
                    &mut FormatArguments::new(&[])
                )
            );
        }

        #[test]
        fn asterisks_with_numbers() {
            assert_eq!(
                Some(42),
                resolve_asterisk_precision(
                    Some(CanAsterisk::Asterisk(ArgumentLocation::NextArgument)),
                    &mut FormatArguments::new(&[FormatArgument::SignedInt(42)]),
                )
            );
            assert_eq!(
                Some(42),
                resolve_asterisk_precision(
                    Some(CanAsterisk::Asterisk(ArgumentLocation::NextArgument)),
                    &mut FormatArguments::new(&[FormatArgument::Unparsed("42".into())]),
                )
            );

            assert_eq!(
                Some(0),
                resolve_asterisk_precision(
                    Some(CanAsterisk::Asterisk(ArgumentLocation::NextArgument)),
                    &mut FormatArguments::new(&[FormatArgument::SignedInt(-42)]),
                )
            );
            assert_eq!(
                Some(0),
                resolve_asterisk_precision(
                    Some(CanAsterisk::Asterisk(ArgumentLocation::NextArgument)),
                    &mut FormatArguments::new(&[FormatArgument::Unparsed("-42".into())]),
                )
            );
            assert_eq!(
                Some(2),
                resolve_asterisk_precision(
                    Some(CanAsterisk::Asterisk(ArgumentLocation::Position(
                        NonZero::new(2).unwrap()
                    ))),
                    &mut FormatArguments::new(&[
                        FormatArgument::Unparsed("1".into()),
                        FormatArgument::Unparsed("2".into()),
                        FormatArgument::Unparsed("3".into())
                    ]),
                )
            );
        }
    }
}
