// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Utilities for parsing numbers in various formats

// spell-checker:ignore powf copysign prec inity

/// Base for number parsing
#[derive(Clone, Copy, PartialEq)]
pub enum Base {
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
pub enum ParseError<'a, T> {
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

impl<'a, T> ParseError<'a, T> {
    fn map<U>(self, f: impl FnOnce(T, &'a str) -> ParseError<'a, U>) -> ParseError<'a, U> {
        match self {
            Self::NotNumeric => ParseError::NotNumeric,
            Self::Overflow => ParseError::Overflow,
            Self::PartialMatch(v, s) => f(v, s),
        }
    }
}

/// A number parser for binary, octal, decimal, hexadecimal and single characters.
///
/// Internally, in order to get the maximum possible precision and cover the full
/// range of u64 and i64 without losing precision for f64, the returned number is
/// decomposed into:
///   - A `base` value
///   - A `neg` sign bit
///   - A `integral` positive part
///   - A `fractional` positive part
///   - A `precision` representing the number of digits in the fractional part
///
/// If the fractional part cannot be represented on a `u64`, parsing continues
/// silently by ignoring non-significant digits.
pub struct ParsedNumber {
    base: Base,
    negative: bool,
    integral: u64,
    fractional: u64,
    precision: usize,
}

impl ParsedNumber {
    fn into_i64(self) -> Option<i64> {
        if self.negative {
            i64::try_from(-i128::from(self.integral)).ok()
        } else {
            i64::try_from(self.integral).ok()
        }
    }

    /// Parse a number as i64. No fractional part is allowed.
    pub fn parse_i64(input: &str) -> Result<i64, ParseError<'_, i64>> {
        match Self::parse(input, true) {
            Ok(v) => v.into_i64().ok_or(ParseError::Overflow),
            Err(e) => Err(e.map(|v, rest| {
                v.into_i64()
                    .map(|v| ParseError::PartialMatch(v, rest))
                    .unwrap_or(ParseError::Overflow)
            })),
        }
    }

    /// Parse a number as u64. No fractional part is allowed.
    pub fn parse_u64(input: &str) -> Result<u64, ParseError<'_, u64>> {
        match Self::parse(input, true) {
            Ok(v) | Err(ParseError::PartialMatch(v, _)) if v.negative => {
                Err(ParseError::NotNumeric)
            }
            Ok(v) => Ok(v.integral),
            Err(e) => Err(e.map(|v, rest| ParseError::PartialMatch(v.integral, rest))),
        }
    }

    fn into_f64(self) -> f64 {
        let n = self.integral as f64
            + (self.fractional as f64) / (self.base as u8 as f64).powf(self.precision as f64);
        if self.negative {
            -n
        } else {
            n
        }
    }

    /// Parse a number as f64
    pub fn parse_f64(input: &str) -> Result<f64, ParseError<'_, f64>> {
        match Self::parse(input, false) {
            Ok(v) => Ok(v.into_f64()),
            Err(ParseError::NotNumeric) => Self::parse_f64_special_values(input),
            Err(e) => Err(e.map(|v, rest| ParseError::PartialMatch(v.into_f64(), rest))),
        }
    }

    fn parse_f64_special_values(input: &str) -> Result<f64, ParseError<'_, f64>> {
        let (sign, rest) = if let Some(input) = input.strip_prefix('-') {
            (-1.0, input)
        } else {
            (1.0, input)
        };
        let prefix = rest
            .chars()
            .take(3)
            .map(|c| c.to_ascii_lowercase())
            .collect::<String>();
        let special = match prefix.as_str() {
            "inf" => f64::INFINITY,
            "nan" => f64::NAN,
            _ => return Err(ParseError::NotNumeric),
        }
        .copysign(sign);
        if rest.len() == 3 {
            Ok(special)
        } else {
            Err(ParseError::PartialMatch(special, &rest[3..]))
        }
    }

    #[allow(clippy::cognitive_complexity)]
    fn parse(input: &str, integral_only: bool) -> Result<Self, ParseError<'_, Self>> {
        // Parse the "'" prefix separately
        if let Some(rest) = input.strip_prefix('\'') {
            let mut chars = rest.char_indices().fuse();
            let v = chars.next().map(|(_, c)| Self {
                base: Base::Decimal,
                negative: false,
                integral: u64::from(c),
                fractional: 0,
                precision: 0,
            });
            return match (v, chars.next()) {
                (Some(v), None) => Ok(v),
                (Some(v), Some((i, _))) => Err(ParseError::PartialMatch(v, &rest[i..])),
                (None, _) => Err(ParseError::NotNumeric),
            };
        }

        // Initial minus sign
        let (negative, unsigned) = if let Some(input) = input.strip_prefix('-') {
            (true, input)
        } else {
            (false, input)
        };

        // Parse an optional base prefix ("0b" / "0B" / "0" / "0x" / "0X"). "0" is octal unless a
        // fractional part is allowed in which case it is an insignificant leading 0. A "0" prefix
        // will not be consumed in case the parsable string contains only "0": the leading extra "0"
        // will have no influence on the result.
        let (base, rest) = if let Some(rest) = unsigned.strip_prefix('0') {
            if let Some(rest) = rest.strip_prefix(['b', 'B']) {
                (Base::Binary, rest)
            } else if let Some(rest) = rest.strip_prefix(['x', 'X']) {
                (Base::Hexadecimal, rest)
            } else if integral_only {
                (Base::Octal, unsigned)
            } else {
                (Base::Decimal, unsigned)
            }
        } else {
            (Base::Decimal, unsigned)
        };
        if rest.is_empty() {
            return Err(ParseError::NotNumeric);
        }

        // Parse the integral part of the number
        let mut chars = rest.chars().enumerate().fuse().peekable();
        let mut integral = 0u64;
        while let Some(d) = chars.peek().and_then(|&(_, c)| base.digit(c)) {
            chars.next();
            integral = integral
                .checked_mul(base as u64)
                .and_then(|n| n.checked_add(d))
                .ok_or(ParseError::Overflow)?;
        }

        // Parse the fractional part of the number if there can be one and the input contains
        // a '.' decimal separator.
        let (mut fractional, mut precision) = (0u64, 0);
        if matches!(chars.peek(), Some(&(_, '.')))
            && matches!(base, Base::Decimal | Base::Hexadecimal)
            && !integral_only
        {
            chars.next();
            let mut ended = false;
            while let Some(d) = chars.peek().and_then(|&(_, c)| base.digit(c)) {
                chars.next();
                if !ended {
                    if let Some(f) = fractional
                        .checked_mul(base as u64)
                        .and_then(|n| n.checked_add(d))
                    {
                        (fractional, precision) = (f, precision + 1);
                    } else {
                        ended = true;
                    }
                }
            }
        }

        // If nothing has been parsed, declare the parsing unsuccessful
        if let Some((0, _)) = chars.peek() {
            return Err(ParseError::NotNumeric);
        }

        // Return what has been parsed so far. It there are extra characters, mark the
        // parsing as a partial match.
        let parsed = Self {
            base,
            negative,
            integral,
            fractional,
            precision,
        };
        if let Some((first_unparsed, _)) = chars.next() {
            Err(ParseError::PartialMatch(parsed, &rest[first_unparsed..]))
        } else {
            Ok(parsed)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ParseError, ParsedNumber};

    #[test]
    fn test_decimal_u64() {
        assert_eq!(Ok(123), ParsedNumber::parse_u64("123"));
        assert_eq!(
            Ok(u64::MAX),
            ParsedNumber::parse_u64(&format!("{}", u64::MAX))
        );
        assert!(matches!(
            ParsedNumber::parse_u64("-123"),
            Err(ParseError::NotNumeric)
        ));
        assert!(matches!(
            ParsedNumber::parse_u64(""),
            Err(ParseError::NotNumeric)
        ));
        assert!(matches!(
            ParsedNumber::parse_u64("123.15"),
            Err(ParseError::PartialMatch(123, ".15"))
        ));
    }

    #[test]
    fn test_decimal_i64() {
        assert_eq!(Ok(123), ParsedNumber::parse_i64("123"));
        assert_eq!(Ok(-123), ParsedNumber::parse_i64("-123"));
        assert!(matches!(
            ParsedNumber::parse_i64("--123"),
            Err(ParseError::NotNumeric)
        ));
        assert_eq!(
            Ok(i64::MAX),
            ParsedNumber::parse_i64(&format!("{}", i64::MAX))
        );
        assert_eq!(
            Ok(i64::MIN),
            ParsedNumber::parse_i64(&format!("{}", i64::MIN))
        );
        assert!(matches!(
            ParsedNumber::parse_i64(&format!("{}", u64::MAX)),
            Err(ParseError::Overflow)
        ));
        assert!(matches!(
            ParsedNumber::parse_i64(&format!("{}", i64::MAX as u64 + 1)),
            Err(ParseError::Overflow)
        ));
    }

    #[test]
    fn test_decimal_f64() {
        assert_eq!(Ok(123.0), ParsedNumber::parse_f64("123"));
        assert_eq!(Ok(-123.0), ParsedNumber::parse_f64("-123"));
        assert_eq!(Ok(123.0), ParsedNumber::parse_f64("123."));
        assert_eq!(Ok(-123.0), ParsedNumber::parse_f64("-123."));
        assert_eq!(Ok(123.0), ParsedNumber::parse_f64("123.0"));
        assert_eq!(Ok(-123.0), ParsedNumber::parse_f64("-123.0"));
        assert_eq!(Ok(123.15), ParsedNumber::parse_f64("123.15"));
        assert_eq!(Ok(-123.15), ParsedNumber::parse_f64("-123.15"));
        assert_eq!(Ok(0.15), ParsedNumber::parse_f64(".15"));
        assert_eq!(Ok(-0.15), ParsedNumber::parse_f64("-.15"));
        assert_eq!(
            Ok(0.15),
            ParsedNumber::parse_f64(".150000000000000000000000000231313")
        );
        assert!(matches!(ParsedNumber::parse_f64("1.2.3"),
                         Err(ParseError::PartialMatch(f, ".3")) if f == 1.2));
        assert_eq!(Ok(f64::INFINITY), ParsedNumber::parse_f64("inf"));
        assert_eq!(Ok(f64::NEG_INFINITY), ParsedNumber::parse_f64("-inf"));
        assert!(ParsedNumber::parse_f64("NaN").unwrap().is_nan());
        assert!(ParsedNumber::parse_f64("NaN").unwrap().is_sign_positive());
        assert!(ParsedNumber::parse_f64("-NaN").unwrap().is_nan());
        assert!(ParsedNumber::parse_f64("-NaN").unwrap().is_sign_negative());
        assert!(matches!(ParsedNumber::parse_f64("-infinity"),
                         Err(ParseError::PartialMatch(f, "inity")) if f == f64::NEG_INFINITY));
        assert!(ParsedNumber::parse_f64(&format!("{}", u64::MAX)).is_ok());
        assert!(ParsedNumber::parse_f64(&format!("{}", i64::MIN)).is_ok());
    }

    #[test]
    fn test_hexadecimal() {
        assert_eq!(Ok(0x123), ParsedNumber::parse_u64("0x123"));
        assert_eq!(Ok(0x123), ParsedNumber::parse_u64("0X123"));
        assert_eq!(Ok(0xfe), ParsedNumber::parse_u64("0xfE"));
        assert_eq!(Ok(-0x123), ParsedNumber::parse_i64("-0x123"));

        assert_eq!(Ok(0.5), ParsedNumber::parse_f64("0x.8"));
        assert_eq!(Ok(0.0625), ParsedNumber::parse_f64("0x.1"));
        assert_eq!(Ok(15.0078125), ParsedNumber::parse_f64("0xf.02"));
    }

    #[test]
    fn test_octal() {
        assert_eq!(Ok(0), ParsedNumber::parse_u64("0"));
        assert_eq!(Ok(0o123), ParsedNumber::parse_u64("0123"));
        assert_eq!(Ok(0o123), ParsedNumber::parse_u64("00123"));
        assert_eq!(Ok(0), ParsedNumber::parse_u64("00"));
        assert!(matches!(
            ParsedNumber::parse_u64("008"),
            Err(ParseError::PartialMatch(0, "8"))
        ));
        assert!(matches!(
            ParsedNumber::parse_u64("08"),
            Err(ParseError::PartialMatch(0, "8"))
        ));
        assert!(matches!(
            ParsedNumber::parse_u64("0."),
            Err(ParseError::PartialMatch(0, "."))
        ));
    }

    #[test]
    fn test_binary() {
        assert_eq!(Ok(0b1011), ParsedNumber::parse_u64("0b1011"));
        assert_eq!(Ok(0b1011), ParsedNumber::parse_u64("0B1011"));
    }
}
