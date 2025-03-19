// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Utilities for parsing numbers in various formats

// spell-checker:ignore powf copysign prec inity bigdecimal extendedbigdecimal biguint

use bigdecimal::{
    BigDecimal,
    num_bigint::{BigInt, BigUint, Sign},
};
use num_traits::ToPrimitive;
use num_traits::Zero;

use crate::format::extendedbigdecimal::ExtendedBigDecimal;

/// Base for number parsing
#[derive(Clone, Copy, PartialEq)]
enum Base {
    /// Binary base
    Binary = 2,

    /// Octal base
    Octal = 8,

    /// Decimal base
    Decimal = 10,

    /// Hexadecimal base
    Hexadecimal = 16,
}

impl Base {
    /// Return the digit value of a character in the given base
    pub fn digit(&self, c: char) -> Option<u64> {
        fn from_decimal(c: char) -> u64 {
            u64::from(c) - u64::from('0')
        }
        match self {
            Self::Binary => ('0'..='1').contains(&c).then(|| from_decimal(c)),
            Self::Octal => ('0'..='7').contains(&c).then(|| from_decimal(c)),
            Self::Decimal => c.is_ascii_digit().then(|| from_decimal(c)),
            Self::Hexadecimal => match c.to_ascii_lowercase() {
                '0'..='9' => Some(from_decimal(c)),
                c @ 'a'..='f' => Some(u64::from(c) - u64::from('a') + 10),
                _ => None,
            },
        }
    }
}

/// Type returned if a number could not be parsed in its entirety
#[derive(Debug, PartialEq)]
pub enum ExtendedParserError<'a, T> {
    /// The input as a whole makes no sense
    NotNumeric,
    /// The beginning of the input made sense and has been parsed,
    /// while the remaining doesn't.
    PartialMatch(T, &'a str),
    /// The integral part has overflowed the requested type, or
    /// has overflowed the `u64` internal storage when parsing the
    /// integral part of a floating point number.
    Overflow,
}

impl<'a, T> ExtendedParserError<'a, T> {
    fn map<U>(
        self,
        f: impl FnOnce(T, &'a str) -> ExtendedParserError<'a, U>,
    ) -> ExtendedParserError<'a, U> {
        match self {
            Self::NotNumeric => ExtendedParserError::NotNumeric,
            Self::Overflow => ExtendedParserError::Overflow,
            Self::PartialMatch(v, s) => f(v, s),
        }
    }
}

/// A number parser for binary, octal, decimal, hexadecimal and single characters.
///
/// It is implemented for `u64`/`i64`, where no fractional part is parsed,
/// and `f64` float, where octal and binary formats are not allowed.
pub trait ExtendedParser {
    // We pick a hopefully different name for our parser, to avoid clash with standard traits.
    fn extended_parse(input: &str) -> Result<Self, ExtendedParserError<'_, Self>>
    where
        Self: Sized;
}

impl ExtendedParser for i64 {
    /// Parse a number as i64. No fractional part is allowed.
    fn extended_parse(input: &str) -> Result<i64, ExtendedParserError<'_, i64>> {
        fn into_i64(ebd: ExtendedBigDecimal) -> Option<i64> {
            match ebd {
                ExtendedBigDecimal::BigDecimal(bd) => {
                    let (digits, scale) = bd.into_bigint_and_scale();
                    if scale == 0 {
                        i64::try_from(digits).ok()
                    } else {
                        None
                    }
                }
                ExtendedBigDecimal::MinusZero => Some(0),
                _ => None,
            }
        }

        match parse(input, true) {
            Ok(v) => into_i64(v).ok_or(ExtendedParserError::Overflow),
            Err(e) => Err(e.map(|v, rest| {
                into_i64(v)
                    .map(|v| ExtendedParserError::PartialMatch(v, rest))
                    .unwrap_or(ExtendedParserError::Overflow)
            })),
        }
    }
}

impl ExtendedParser for u64 {
    /// Parse a number as u64. No fractional part is allowed.
    fn extended_parse(input: &str) -> Result<u64, ExtendedParserError<'_, u64>> {
        fn into_u64(ebd: ExtendedBigDecimal) -> Option<u64> {
            match ebd {
                ExtendedBigDecimal::BigDecimal(bd) => {
                    let (digits, scale) = bd.into_bigint_and_scale();
                    if scale == 0 {
                        u64::try_from(digits).ok()
                    } else {
                        None
                    }
                }
                _ => None,
            }
        }

        match parse(input, true) {
            Ok(v) => into_u64(v).ok_or(ExtendedParserError::Overflow),
            Err(e) => Err(e.map(|v, rest| {
                into_u64(v)
                    .map(|v| ExtendedParserError::PartialMatch(v, rest))
                    .unwrap_or(ExtendedParserError::Overflow)
            })),
        }
    }
}

impl ExtendedParser for f64 {
    /// Parse a number as f64
    fn extended_parse(input: &str) -> Result<f64, ExtendedParserError<'_, f64>> {
        // TODO: This is generic, so this should probably be implemented as an ExtendedBigDecimal trait (ToPrimitive).
        fn into_f64(ebd: ExtendedBigDecimal) -> f64 {
            match ebd {
                ExtendedBigDecimal::BigDecimal(bd) => bd.to_f64().unwrap(),
                ExtendedBigDecimal::MinusZero => -0.0,
                ExtendedBigDecimal::Nan => f64::NAN,
                ExtendedBigDecimal::MinusNan => -f64::NAN,
                ExtendedBigDecimal::Infinity => f64::INFINITY,
                ExtendedBigDecimal::MinusInfinity => -f64::INFINITY,
            }
        }

        match parse(input, false) {
            Ok(v) => Ok(into_f64(v)),
            Err(e) => Err(e.map(|v, rest| ExtendedParserError::PartialMatch(into_f64(v), rest))),
        }
    }
}

impl ExtendedParser for ExtendedBigDecimal {
    /// Parse a number as an ExtendedBigDecimal
    fn extended_parse(
        input: &str,
    ) -> Result<ExtendedBigDecimal, ExtendedParserError<'_, ExtendedBigDecimal>> {
        parse(input, false)
    }
}

fn parse_special_value(
    input: &str,
    negative: bool,
) -> Result<ExtendedBigDecimal, ExtendedParserError<'_, ExtendedBigDecimal>> {
    let prefix = input
        .chars()
        .take(3)
        .map(|c| c.to_ascii_lowercase())
        .collect::<String>();
    let special = match prefix.as_str() {
        "inf" => {
            if negative {
                ExtendedBigDecimal::MinusInfinity
            } else {
                ExtendedBigDecimal::Infinity
            }
        }
        "nan" => {
            if negative {
                ExtendedBigDecimal::MinusNan
            } else {
                ExtendedBigDecimal::Nan
            }
        }
        _ => return Err(ExtendedParserError::NotNumeric),
    };
    if input.len() == 3 {
        Ok(special)
    } else {
        Err(ExtendedParserError::PartialMatch(special, &input[3..]))
    }
}

#[allow(clippy::cognitive_complexity)]
fn parse(
    input: &str,
    integral_only: bool,
) -> Result<ExtendedBigDecimal, ExtendedParserError<'_, ExtendedBigDecimal>> {
    // Parse the "'" prefix separately
    if let Some(rest) = input.strip_prefix('\'') {
        let mut chars = rest.char_indices().fuse();
        let v = chars
            .next()
            .map(|(_, c)| ExtendedBigDecimal::BigDecimal(u32::from(c).into()));
        return match (v, chars.next()) {
            (Some(v), None) => Ok(v),
            (Some(v), Some((i, _))) => Err(ExtendedParserError::PartialMatch(v, &rest[i..])),
            (None, _) => Err(ExtendedParserError::NotNumeric),
        };
    }

    let trimmed_input = input.trim_ascii_start();

    // Initial minus/plus sign
    let (negative, unsigned) = if let Some(trimmed_input) = trimmed_input.strip_prefix('-') {
        (true, trimmed_input)
    } else if let Some(trimmed_input) = trimmed_input.strip_prefix('+') {
        (false, trimmed_input)
    } else {
        (false, trimmed_input)
    };

    // Parse an optional base prefix ("0b" / "0B" / "0" / "0x" / "0X"). "0" is octal unless a
    // fractional part is allowed in which case it is an insignificant leading 0. A "0" prefix
    // will not be consumed in case the parsable string contains only "0": the leading extra "0"
    // will have no influence on the result.
    let (base, rest) = if let Some(rest) = unsigned.strip_prefix('0') {
        if let Some(rest) = rest.strip_prefix(['x', 'X']) {
            (Base::Hexadecimal, rest)
        } else if integral_only {
            // Binary/Octal only allowed for integer parsing.
            if let Some(rest) = rest.strip_prefix(['b', 'B']) {
                (Base::Binary, rest)
            } else {
                (Base::Octal, unsigned)
            }
        } else {
            (Base::Decimal, unsigned)
        }
    } else {
        (Base::Decimal, unsigned)
    };
    if rest.is_empty() {
        return Err(ExtendedParserError::NotNumeric);
    }

    // Parse the integral part of the number
    let mut chars = rest.chars().enumerate().fuse().peekable();
    let mut digits = BigUint::zero();
    let mut scale = 0i64;
    while let Some(d) = chars.peek().and_then(|&(_, c)| base.digit(c)) {
        chars.next();
        digits = digits * base as u8 + d;
    }

    // Parse the fractional part of the number if there can be one and the input contains
    // a '.' decimal separator.
    if matches!(chars.peek(), Some(&(_, '.')))
        && matches!(base, Base::Decimal | Base::Hexadecimal)
        && !integral_only
    {
        chars.next();
        while let Some(d) = chars.peek().and_then(|&(_, c)| base.digit(c)) {
            chars.next();
            (digits, scale) = (digits * base as u8 + d, scale + 1);
        }
    }

    // If nothing has been parsed, check if this is a special value, or declare the parsing unsuccessful
    if let Some((0, _)) = chars.peek() {
        if integral_only {
            return Err(ExtendedParserError::NotNumeric);
        } else {
            return parse_special_value(unsigned, negative);
        }
    }

    // TODO: Might be nice to implement a ExtendedBigDecimal copysign or negation function to move away some of this logic...
    let ebd = if digits == BigUint::zero() && negative {
        ExtendedBigDecimal::MinusZero
    } else {
        let sign = if negative { Sign::Minus } else { Sign::Plus };
        let signed_digits = BigInt::from_biguint(sign, digits);
        let bd = if scale == 0 {
            BigDecimal::from_bigint(signed_digits, 0)
        } else if base == Base::Decimal {
            BigDecimal::from_bigint(signed_digits, scale)
        } else {
            // Base is not 10, init at scale 0 then divide by base**scale.
            BigDecimal::from_bigint(signed_digits, 0)
                / BigDecimal::from_bigint(BigInt::from(base as u32).pow(scale as u32), 0)
        };
        ExtendedBigDecimal::BigDecimal(bd)
    };

    // Return what has been parsed so far. It there are extra characters, mark the
    // parsing as a partial match.
    if let Some((first_unparsed, _)) = chars.next() {
        Err(ExtendedParserError::PartialMatch(
            ebd,
            &rest[first_unparsed..],
        ))
    } else {
        Ok(ebd)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use bigdecimal::BigDecimal;

    use crate::format::ExtendedBigDecimal;

    use super::{ExtendedParser, ExtendedParserError};

    #[test]
    fn test_decimal_u64() {
        assert_eq!(Ok(123), u64::extended_parse("123"));
        assert_eq!(Ok(u64::MAX), u64::extended_parse(&format!("{}", u64::MAX)));
        assert!(matches!(
            u64::extended_parse("-123"),
            Err(ExtendedParserError::Overflow)
        ));
        assert!(matches!(
            u64::extended_parse(""),
            Err(ExtendedParserError::NotNumeric)
        ));
        assert!(matches!(
            u64::extended_parse("123.15"),
            Err(ExtendedParserError::PartialMatch(123, ".15"))
        ));
    }

    #[test]
    fn test_decimal_i64() {
        assert_eq!(Ok(123), i64::extended_parse("123"));
        assert_eq!(Ok(123), i64::extended_parse("+123"));
        assert_eq!(Ok(-123), i64::extended_parse("-123"));
        assert!(matches!(
            i64::extended_parse("--123"),
            Err(ExtendedParserError::NotNumeric)
        ));
        assert_eq!(Ok(i64::MAX), i64::extended_parse(&format!("{}", i64::MAX)));
        assert_eq!(Ok(i64::MIN), i64::extended_parse(&format!("{}", i64::MIN)));
        assert!(matches!(
            i64::extended_parse(&format!("{}", u64::MAX)),
            Err(ExtendedParserError::Overflow)
        ));
        assert!(matches!(
            i64::extended_parse(&format!("{}", i64::MAX as u64 + 1)),
            Err(ExtendedParserError::Overflow)
        ));
    }

    #[test]
    fn test_decimal_f64() {
        assert_eq!(Ok(123.0), f64::extended_parse("123"));
        assert_eq!(Ok(123.0), f64::extended_parse("+123"));
        assert_eq!(Ok(-123.0), f64::extended_parse("-123"));
        assert_eq!(Ok(123.0), f64::extended_parse("123."));
        assert_eq!(Ok(-123.0), f64::extended_parse("-123."));
        assert_eq!(Ok(123.0), f64::extended_parse("123.0"));
        assert_eq!(Ok(-123.0), f64::extended_parse("-123.0"));
        assert_eq!(Ok(123.15), f64::extended_parse("123.15"));
        assert_eq!(Ok(123.15), f64::extended_parse("+123.15"));
        assert_eq!(Ok(-123.15), f64::extended_parse("-123.15"));
        assert_eq!(Ok(0.15), f64::extended_parse(".15"));
        assert_eq!(Ok(-0.15), f64::extended_parse("-.15"));
        // Leading 0(s) are _not_ octal when parsed as float
        assert_eq!(Ok(123.0), f64::extended_parse("0123"));
        assert_eq!(Ok(123.0), f64::extended_parse("+0123"));
        assert_eq!(Ok(-123.0), f64::extended_parse("-0123"));
        assert_eq!(Ok(123.0), f64::extended_parse("00123"));
        assert_eq!(Ok(123.0), f64::extended_parse("+00123"));
        assert_eq!(Ok(-123.0), f64::extended_parse("-00123"));
        assert_eq!(Ok(123.15), f64::extended_parse("0123.15"));
        assert_eq!(Ok(123.15), f64::extended_parse("+0123.15"));
        assert_eq!(Ok(-123.15), f64::extended_parse("-0123.15"));
        assert_eq!(
            Ok(0.15),
            f64::extended_parse(".150000000000000000000000000231313")
        );
        assert!(matches!(f64::extended_parse("1.2.3"),
                         Err(ExtendedParserError::PartialMatch(f, ".3")) if f == 1.2));
        // Minus zero. 0.0 == -0.0 so we explicitly check the sign.
        assert_eq!(Ok(0.0), f64::extended_parse("-0.0"));
        assert!(f64::extended_parse("-0.0").unwrap().is_sign_negative());
        assert_eq!(Ok(f64::INFINITY), f64::extended_parse("inf"));
        assert_eq!(Ok(f64::INFINITY), f64::extended_parse("+inf"));
        assert_eq!(Ok(f64::NEG_INFINITY), f64::extended_parse("-inf"));
        assert_eq!(Ok(f64::INFINITY), f64::extended_parse("Inf"));
        assert_eq!(Ok(f64::INFINITY), f64::extended_parse("InF"));
        assert_eq!(Ok(f64::INFINITY), f64::extended_parse("INF"));
        assert!(f64::extended_parse("NaN").unwrap().is_nan());
        assert!(f64::extended_parse("NaN").unwrap().is_sign_positive());
        assert!(f64::extended_parse("+NaN").unwrap().is_nan());
        assert!(f64::extended_parse("+NaN").unwrap().is_sign_positive());
        assert!(f64::extended_parse("-NaN").unwrap().is_nan());
        assert!(f64::extended_parse("-NaN").unwrap().is_sign_negative());
        assert!(f64::extended_parse("nan").unwrap().is_nan());
        assert!(f64::extended_parse("nan").unwrap().is_sign_positive());
        assert!(f64::extended_parse("NAN").unwrap().is_nan());
        assert!(f64::extended_parse("NAN").unwrap().is_sign_positive());
        assert!(matches!(f64::extended_parse("-infinity"),
                         Err(ExtendedParserError::PartialMatch(f, "inity")) if f == f64::NEG_INFINITY));
        assert!(f64::extended_parse(&format!("{}", u64::MAX)).is_ok());
        assert!(f64::extended_parse(&format!("{}", i64::MIN)).is_ok());
    }

    #[test]
    fn test_decimal_extended_big_decimal() {
        // f64 parsing above already tested a lot of these, just do a few.
        // Careful, we usually cannot use From<f64> to get a precise ExtendedBigDecimal as numbers like 123.15 cannot be exactly represented by a f64.
        assert_eq!(
            Ok(ExtendedBigDecimal::BigDecimal(
                BigDecimal::from_str("123").unwrap()
            )),
            ExtendedBigDecimal::extended_parse("123")
        );
        assert_eq!(
            Ok(ExtendedBigDecimal::BigDecimal(
                BigDecimal::from_str("123.15").unwrap()
            )),
            ExtendedBigDecimal::extended_parse("123.15")
        );
        // Very high precision that would not fit in a f64.
        assert_eq!(
            Ok(ExtendedBigDecimal::BigDecimal(
                BigDecimal::from_str(".150000000000000000000000000000000000001").unwrap()
            )),
            ExtendedBigDecimal::extended_parse(".150000000000000000000000000000000000001")
        );
        assert!(matches!(
            ExtendedBigDecimal::extended_parse("nan"),
            Ok(ExtendedBigDecimal::Nan)
        ));
        assert!(matches!(
            ExtendedBigDecimal::extended_parse("-NAN"),
            Ok(ExtendedBigDecimal::MinusNan)
        ));
        assert_eq!(
            Ok(ExtendedBigDecimal::Infinity),
            ExtendedBigDecimal::extended_parse("InF")
        );
        assert_eq!(
            Ok(ExtendedBigDecimal::MinusInfinity),
            ExtendedBigDecimal::extended_parse("-iNf")
        );
        assert_eq!(
            Ok(ExtendedBigDecimal::zero()),
            ExtendedBigDecimal::extended_parse("0.0")
        );
        assert!(matches!(
            ExtendedBigDecimal::extended_parse("-0.0"),
            Ok(ExtendedBigDecimal::MinusZero)
        ));
    }

    #[test]
    fn test_hexadecimal() {
        assert_eq!(Ok(0x123), u64::extended_parse("0x123"));
        assert_eq!(Ok(0x123), u64::extended_parse("0X123"));
        assert_eq!(Ok(0x123), u64::extended_parse("+0x123"));
        assert_eq!(Ok(0xfe), u64::extended_parse("0xfE"));
        assert_eq!(Ok(-0x123), i64::extended_parse("-0x123"));

        assert_eq!(Ok(0.5), f64::extended_parse("0x.8"));
        assert_eq!(Ok(0.0625), f64::extended_parse("0x.1"));
        assert_eq!(Ok(15.007_812_5), f64::extended_parse("0xf.02"));

        assert_eq!(
            Ok(ExtendedBigDecimal::BigDecimal(
                BigDecimal::from_str("0.0625").unwrap()
            )),
            ExtendedBigDecimal::extended_parse("0x.1")
        );

        // Precisely parse very large hexadecimal numbers (i.e. with a large division).
        assert_eq!(
            Ok(ExtendedBigDecimal::BigDecimal(
                BigDecimal::from_str("15.999999999999999999999999948301211715435770320536956745627321652136743068695068359375").unwrap()
            )),
            ExtendedBigDecimal::extended_parse("0xf.fffffffffffffffffffff")
        );
    }

    #[test]
    fn test_octal() {
        assert_eq!(Ok(0), u64::extended_parse("0"));
        assert_eq!(Ok(0o123), u64::extended_parse("0123"));
        assert_eq!(Ok(0o123), u64::extended_parse("+0123"));
        assert_eq!(Ok(-0o123), i64::extended_parse("-0123"));
        assert_eq!(Ok(0o123), u64::extended_parse("00123"));
        assert_eq!(Ok(0), u64::extended_parse("00"));
        assert!(matches!(
            u64::extended_parse("008"),
            Err(ExtendedParserError::PartialMatch(0, "8"))
        ));
        assert!(matches!(
            u64::extended_parse("08"),
            Err(ExtendedParserError::PartialMatch(0, "8"))
        ));
        assert!(matches!(
            u64::extended_parse("0."),
            Err(ExtendedParserError::PartialMatch(0, "."))
        ));

        // No float tests, leading zeros get parsed as decimal anyway.
    }

    #[test]
    fn test_binary() {
        assert_eq!(Ok(0b1011), u64::extended_parse("0b1011"));
        assert_eq!(Ok(0b1011), u64::extended_parse("0B1011"));
        assert_eq!(Ok(0b1011), u64::extended_parse("+0b1011"));
        assert_eq!(Ok(-0b1011), i64::extended_parse("-0b1011"));

        // Binary not allowed for floats
        assert!(matches!(
            f64::extended_parse("0b100"),
            Err(ExtendedParserError::PartialMatch(0f64, "b100"))
        ));
        assert!(matches!(
            f64::extended_parse("0b100.1"),
            Err(ExtendedParserError::PartialMatch(0f64, "b100.1"))
        ));

        assert!(match ExtendedBigDecimal::extended_parse("0b100.1") {
            Err(ExtendedParserError::PartialMatch(ebd, "b100.1")) =>
                ebd == ExtendedBigDecimal::zero(),
            _ => false,
        });
    }

    #[test]
    fn test_parsing_with_leading_whitespace() {
        assert_eq!(Ok(1), u64::extended_parse(" 0x1"));
        assert_eq!(Ok(-2), i64::extended_parse(" -0x2"));
        assert_eq!(Ok(-3), i64::extended_parse(" \t-0x3"));
        assert_eq!(Ok(-4), i64::extended_parse(" \n-0x4"));
        assert_eq!(Ok(-5), i64::extended_parse(" \n\t\u{000d}-0x5"));

        // Ensure that trailing whitespace is still a partial match
        assert_eq!(
            Err(ExtendedParserError::PartialMatch(6, " ")),
            u64::extended_parse("0x6 ")
        );
        assert_eq!(
            Err(ExtendedParserError::PartialMatch(7, "\t")),
            u64::extended_parse("0x7\t")
        );
        assert_eq!(
            Err(ExtendedParserError::PartialMatch(8, "\n")),
            u64::extended_parse("0x8\n")
        );

        // Ensure that unicode non-ascii whitespace is a partial match
        assert_eq!(
            Err(ExtendedParserError::NotNumeric),
            i64::extended_parse("\u{2029}-0x9")
        );

        // Ensure that whitespace after the number has "started" is not allowed
        assert_eq!(
            Err(ExtendedParserError::NotNumeric),
            i64::extended_parse("- 0x9")
        );
    }
}
