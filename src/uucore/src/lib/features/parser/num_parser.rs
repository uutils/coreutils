// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Utilities for parsing numbers in various formats

// spell-checker:ignore powf copysign prec ilog inity infinit infs bigdecimal extendedbigdecimal biguint underflowed muls

use std::num::NonZeroU64;

use bigdecimal::{
    BigDecimal, Context,
    num_bigint::{BigInt, BigUint, Sign},
};
use num_traits::Signed;
use num_traits::ToPrimitive;
use num_traits::Zero;

use crate::extendedbigdecimal::ExtendedBigDecimal;

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
    fn digit(&self, c: char) -> Option<u64> {
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

    /// Greedily parse as many digits as possible from the string
    /// Returns parsed digits (if any), and the rest of the string.
    fn parse_digits<'a>(&self, str: &'a str) -> (Option<BigUint>, &'a str) {
        let (digits, _, rest) = self.parse_digits_count(str, None);
        (digits, rest)
    }

    /// Greedily parse as many digits as possible from the string, adding to already parsed digits.
    /// This is meant to be used (directly) for the part after a decimal point.
    /// Returns parsed digits (if any), the number of parsed digits, and the rest of the string.
    fn parse_digits_count<'a>(
        &self,
        str: &'a str,
        digits: Option<BigUint>,
    ) -> (Option<BigUint>, i64, &'a str) {
        let mut digits: Option<BigUint> = digits;
        let mut count: i64 = 0;
        let mut rest = str;

        // Doing operations on BigUint is really expensive, so we do as much as we
        // can on u64, then add them to the BigUint.
        let mut digits_tmp: u64 = 0;
        let mut count_tmp: i64 = 0;
        let mut mul_tmp: u64 = 1;
        while let Some(d) = rest.chars().next().and_then(|c| self.digit(c)) {
            (digits_tmp, count_tmp, mul_tmp) = (
                digits_tmp * *self as u64 + d,
                count_tmp + 1,
                mul_tmp * *self as u64,
            );
            rest = &rest[1..];
            // In base 16, we parse 4 bits at a time, so we can parse 16 digits at most in a u64.
            if count_tmp >= 15 {
                // Accumulate what we have so far
                (digits, count) = (
                    Some(digits.unwrap_or_default() * mul_tmp + digits_tmp),
                    count + count_tmp,
                );
                // Reset state
                (digits_tmp, count_tmp, mul_tmp) = (0, 0, 1);
            }
        }

        // Accumulate the leftovers (if any)
        if mul_tmp > 1 {
            (digits, count) = (
                Some(digits.unwrap_or_default() * mul_tmp + digits_tmp),
                count + count_tmp,
            );
        }
        (digits, count, rest)
    }
}

/// Type returned if a number could not be parsed in its entirety
#[derive(Debug, PartialEq)]
pub enum ExtendedParserError<T> {
    /// The input as a whole makes no sense
    NotNumeric,
    /// The beginning of the input made sense and has been parsed,
    /// while the remaining doesn't.
    PartialMatch(T, String),
    /// The value has overflowed the type storage. The returned value
    /// is saturated (e.g. positive or negative infinity, or min/max
    /// value for the integer type).
    Overflow(T),
    // The value has underflowed the float storage (and is now 0.0 or -0.0).
    // Does not apply to integer parsing.
    Underflow(T),
}

impl<T> ExtendedParserError<T>
where
    T: Zero,
{
    /// Extract the value out of an error, if possible.
    fn extract(self) -> T {
        match self {
            Self::NotNumeric => T::zero(),
            Self::PartialMatch(v, _) => v,
            Self::Overflow(v) => v,
            Self::Underflow(v) => v,
        }
    }

    /// Map an error to another, using the provided conversion function.
    /// The error (self) takes precedence over errors happening during the
    /// conversion.
    fn map<U>(
        self,
        f: impl FnOnce(T) -> Result<U, ExtendedParserError<U>>,
    ) -> ExtendedParserError<U>
    where
        U: Zero,
    {
        fn extract<U>(v: Result<U, ExtendedParserError<U>>) -> U
        where
            U: Zero,
        {
            v.unwrap_or_else(|e| e.extract())
        }

        match self {
            ExtendedParserError::NotNumeric => ExtendedParserError::NotNumeric,
            ExtendedParserError::PartialMatch(v, rest) => {
                ExtendedParserError::PartialMatch(extract(f(v)), rest)
            }
            ExtendedParserError::Overflow(v) => ExtendedParserError::Overflow(extract(f(v))),
            ExtendedParserError::Underflow(v) => ExtendedParserError::Underflow(extract(f(v))),
        }
    }
}

/// A number parser for binary, octal, decimal, hexadecimal and single characters.
///
/// It is implemented for `u64`/`i64`, where no fractional part is parsed,
/// and `f64` float, where octal and binary formats are not allowed.
pub trait ExtendedParser {
    // We pick a hopefully different name for our parser, to avoid clash with standard traits.
    fn extended_parse(input: &str) -> Result<Self, ExtendedParserError<Self>>
    where
        Self: Sized;
}

impl ExtendedParser for i64 {
    /// Parse a number as i64. No fractional part is allowed.
    fn extended_parse(input: &str) -> Result<i64, ExtendedParserError<i64>> {
        fn into_i64(ebd: ExtendedBigDecimal) -> Result<i64, ExtendedParserError<i64>> {
            match ebd {
                ExtendedBigDecimal::BigDecimal(bd) => {
                    let (digits, scale) = bd.into_bigint_and_scale();
                    if scale == 0 {
                        let negative = digits.sign() == Sign::Minus;
                        match i64::try_from(digits) {
                            Ok(i) => Ok(i),
                            _ => Err(ExtendedParserError::Overflow(if negative {
                                i64::MIN
                            } else {
                                i64::MAX
                            })),
                        }
                    } else {
                        // Should not happen.
                        Err(ExtendedParserError::NotNumeric)
                    }
                }
                ExtendedBigDecimal::MinusZero => Ok(0),
                // No other case should not happen.
                _ => Err(ExtendedParserError::NotNumeric),
            }
        }

        match parse(input, ParseTarget::Integral, &[]) {
            Ok(v) => into_i64(v),
            Err(e) => Err(e.map(into_i64)),
        }
    }
}

impl ExtendedParser for u64 {
    /// Parse a number as u64. No fractional part is allowed.
    fn extended_parse(input: &str) -> Result<u64, ExtendedParserError<u64>> {
        fn into_u64(ebd: ExtendedBigDecimal) -> Result<u64, ExtendedParserError<u64>> {
            match ebd {
                ExtendedBigDecimal::BigDecimal(bd) => {
                    let (digits, scale) = bd.into_bigint_and_scale();
                    if scale == 0 {
                        let (sign, digits) = digits.into_parts();

                        match u64::try_from(digits) {
                            Ok(i) => {
                                if sign == Sign::Minus {
                                    Ok(!i + 1)
                                } else {
                                    Ok(i)
                                }
                            }
                            _ => Err(ExtendedParserError::Overflow(u64::MAX)),
                        }
                    } else {
                        // Should not happen.
                        Err(ExtendedParserError::NotNumeric)
                    }
                }
                ExtendedBigDecimal::MinusZero => Ok(0),
                _ => Err(ExtendedParserError::NotNumeric),
            }
        }

        match parse(input, ParseTarget::Integral, &[]) {
            Ok(v) => into_u64(v),
            Err(e) => Err(e.map(into_u64)),
        }
    }
}

impl ExtendedParser for f64 {
    /// Parse a number as f64
    fn extended_parse(input: &str) -> Result<f64, ExtendedParserError<f64>> {
        fn into_f64(ebd: ExtendedBigDecimal) -> Result<f64, ExtendedParserError<f64>> {
            // TODO: _Some_ of this is generic, so this should probably be implemented as an ExtendedBigDecimal trait (ToPrimitive).
            let v = match ebd {
                ExtendedBigDecimal::BigDecimal(bd) => {
                    let f = bd.to_f64().unwrap();
                    if f.is_infinite() {
                        return Err(ExtendedParserError::Overflow(f));
                    }
                    if f.is_zero() && !bd.is_zero() {
                        return Err(ExtendedParserError::Underflow(f));
                    }
                    f
                }
                ExtendedBigDecimal::MinusZero => -0.0,
                ExtendedBigDecimal::Nan => f64::NAN,
                ExtendedBigDecimal::MinusNan => -f64::NAN,
                ExtendedBigDecimal::Infinity => f64::INFINITY,
                ExtendedBigDecimal::MinusInfinity => -f64::INFINITY,
            };
            Ok(v)
        }

        match parse(input, ParseTarget::Decimal, &[]) {
            Ok(v) => into_f64(v),
            Err(e) => Err(e.map(into_f64)),
        }
    }
}

impl ExtendedParser for ExtendedBigDecimal {
    /// Parse a number as an ExtendedBigDecimal
    fn extended_parse(
        input: &str,
    ) -> Result<ExtendedBigDecimal, ExtendedParserError<ExtendedBigDecimal>> {
        parse(input, ParseTarget::Decimal, &[])
    }
}

fn parse_digits(base: Base, str: &str, fractional: bool) -> (Option<BigUint>, i64, &str) {
    // Parse the integral part of the number
    let (digits, rest) = base.parse_digits(str);

    // If allowed, parse the fractional part of the number if there can be one and the
    // input contains a '.' decimal separator.
    if fractional {
        if let Some(rest) = rest.strip_prefix('.') {
            return base.parse_digits_count(rest, digits);
        }
    }

    (digits, 0, rest)
}

fn parse_exponent(base: Base, str: &str) -> (Option<BigInt>, &str) {
    let exp_chars = match base {
        Base::Decimal => ['e', 'E'],
        Base::Hexadecimal => ['p', 'P'],
        _ => unreachable!(),
    };

    // Parse the exponent part, only decimal numbers are allowed.
    // We only update `rest` if an exponent is actually parsed.
    if let Some(rest) = str.strip_prefix(exp_chars) {
        let (sign, rest) = if let Some(rest) = rest.strip_prefix('-') {
            (Sign::Minus, rest)
        } else if let Some(rest) = rest.strip_prefix('+') {
            (Sign::Plus, rest)
        } else {
            // Something else, or nothing at all: keep going.
            (Sign::Plus, rest) // No explicit sign is equivalent to `+`.
        };

        let (exp_uint, rest) = Base::Decimal.parse_digits(rest);
        if let Some(exp_uint) = exp_uint {
            return (Some(BigInt::from_biguint(sign, exp_uint)), rest);
        }
    }

    // Nothing parsed
    (None, str)
}

/// Parse a multiplier from allowed suffixes (e.g. s/m/h).
fn parse_suffix_multiplier<'a>(str: &'a str, allowed_suffixes: &[(char, u32)]) -> (u32, &'a str) {
    if let Some(ch) = str.chars().next() {
        if let Some(mul) = allowed_suffixes
            .iter()
            .find_map(|(c, t)| (ch == *c).then_some(*t))
        {
            return (mul, &str[1..]);
        }
    }

    // No suffix, just return 1 and intact string
    (1, str)
}

fn parse_special_value(
    input: &str,
    negative: bool,
    allowed_suffixes: &[(char, u32)],
) -> Result<ExtendedBigDecimal, ExtendedParserError<ExtendedBigDecimal>> {
    let input_lc = input.to_ascii_lowercase();

    // Array of ("String to match", return value when sign positive, when sign negative)
    const MATCH_TABLE: &[(&str, ExtendedBigDecimal)] = &[
        ("infinity", ExtendedBigDecimal::Infinity),
        ("inf", ExtendedBigDecimal::Infinity),
        ("nan", ExtendedBigDecimal::Nan),
    ];

    for (str, ebd) in MATCH_TABLE {
        if input_lc.starts_with(str) {
            let mut special = ebd.clone();
            if negative {
                special = -special;
            }

            // "infs" is a valid duration, so parse suffix multiplier in the original input string, but ignore the multiplier.
            let (_, rest) = parse_suffix_multiplier(&input[str.len()..], allowed_suffixes);

            return if rest.is_empty() {
                Ok(special)
            } else {
                Err(ExtendedParserError::PartialMatch(special, rest.to_string()))
            };
        }
    }

    Err(ExtendedParserError::NotNumeric)
}

/// Underflow/Overflow errors always contain 0 or infinity.
/// overflow: true for overflow, false for underflow.
fn make_error(overflow: bool, negative: bool) -> ExtendedParserError<ExtendedBigDecimal> {
    let mut v = if overflow {
        ExtendedBigDecimal::Infinity
    } else {
        ExtendedBigDecimal::zero()
    };
    if negative {
        v = -v;
    }
    if overflow {
        ExtendedParserError::Overflow(v)
    } else {
        ExtendedParserError::Underflow(v)
    }
}

/// Compute bd**exp using exponentiation by squaring algorithm, while maintaining the
/// precision specified in ctx (the number of digits would otherwise explode).
///
/// Algorithm comes from <https://en.wikipedia.org/wiki/Exponentiation_by_squaring>
///
/// TODO: Still pending discussion in <https://github.com/akubera/bigdecimal-rs/issues/147>,
/// we do lose a little bit of precision, and the last digits may not be correct.
/// Note: This has been copied from the latest revision in <https://github.com/akubera/bigdecimal-rs/pull/148>,
/// so it's using minimum Rust version of `bigdecimal-rs`.
fn pow_with_context(bd: &BigDecimal, exp: i64, ctx: &Context) -> BigDecimal {
    if exp == 0 {
        return 1.into();
    }

    // When performing a multiplication between 2 numbers, we may lose up to 2 digits
    // of precision.
    // "Proof": https://github.com/akubera/bigdecimal-rs/issues/147#issuecomment-2793431202
    const MARGIN_PER_MUL: u64 = 2;
    // When doing many multiplication, we still introduce additional errors, add 1 more digit
    // per 10 multiplications.
    const MUL_PER_MARGIN_EXTRA: u64 = 10;

    fn trim_precision(bd: BigDecimal, ctx: &Context, margin: u64) -> BigDecimal {
        let prec = ctx.precision().get() + margin;
        if bd.digits() > prec {
            bd.with_precision_round(NonZeroU64::new(prec).unwrap(), ctx.rounding_mode())
        } else {
            bd
        }
    }

    // Count the number of multiplications we're going to perform, one per "1" binary digit
    // in exp, and the number of times we can divide exp by 2.
    let mut n = exp.unsigned_abs();
    // Note: 63 - n.leading_zeros() == n.ilog2, but that's only available in recent Rust versions.
    let muls = (n.count_ones() + (63 - n.leading_zeros()) - 1) as u64;
    // Note: div_ceil would be nice to use here, but only available in recent Rust versions.
    // (see note above about minimum Rust version in use)
    let margin_extra = (muls + MUL_PER_MARGIN_EXTRA / 2) / MUL_PER_MARGIN_EXTRA;
    let mut margin = margin_extra + MARGIN_PER_MUL * muls;

    let mut bd_y: BigDecimal = 1.into();
    let mut bd_x = if exp >= 0 {
        bd.clone()
    } else {
        bd.inverse_with_context(&ctx.with_precision(
            NonZeroU64::new(ctx.precision().get() + margin + MARGIN_PER_MUL).unwrap(),
        ))
    };

    while n > 1 {
        if n % 2 == 1 {
            bd_y = trim_precision(&bd_x * bd_y, ctx, margin);
            margin -= MARGIN_PER_MUL;
            n -= 1;
        }
        bd_x = trim_precision(bd_x.square(), ctx, margin);
        margin -= MARGIN_PER_MUL;
        n /= 2;
    }
    debug_assert_eq!(margin, margin_extra);

    trim_precision(bd_x * bd_y, ctx, 0)
}

/// Construct an [`ExtendedBigDecimal`] based on parsed data
fn construct_extended_big_decimal(
    digits: BigUint,
    negative: bool,
    base: Base,
    scale: i64,
    exponent: BigInt,
) -> Result<ExtendedBigDecimal, ExtendedParserError<ExtendedBigDecimal>> {
    if digits == BigUint::zero() {
        // Return return 0 if the digits are zero. In particular, we do not ever
        // return Overflow/Underflow errors in that case.
        return Ok(if negative {
            ExtendedBigDecimal::MinusZero
        } else {
            ExtendedBigDecimal::zero()
        });
    }

    let sign = if negative { Sign::Minus } else { Sign::Plus };
    let signed_digits = BigInt::from_biguint(sign, digits);
    let bd = if scale == 0 && exponent.is_zero() {
        BigDecimal::from_bigint(signed_digits, 0)
    } else if base == Base::Decimal {
        if exponent.is_zero() {
            // Optimization: Converting scale to Bigint and back is relatively slow.
            BigDecimal::from_bigint(signed_digits, scale)
        } else {
            let new_scale = -exponent + scale;

            // BigDecimal "only" supports i64 scale.
            // Note that new_scale is a negative exponent: large positive value causes an underflow, large negative values an overflow.
            if let Some(new_scale) = new_scale.to_i64() {
                BigDecimal::from_bigint(signed_digits, new_scale)
            } else {
                return Err(make_error(new_scale.is_negative(), negative));
            }
        }
    } else if base == Base::Hexadecimal {
        // pow "only" supports u32 values, just error out if given more than 2**32 fractional digits.
        if scale > u32::MAX.into() {
            return Err(ExtendedParserError::NotNumeric);
        }

        // Base is 16, init at scale 0 then divide by base**scale.
        let bd = BigDecimal::from_bigint(signed_digits, 0)
            / BigDecimal::from_bigint(BigInt::from(16).pow(scale as u32), 0);

        // pow_with_context "only" supports i64 values. Just overflow/underflow if the value provided
        // is > 2**64 or < 2**-64.
        let Some(exponent) = exponent.to_i64() else {
            return Err(make_error(exponent.is_positive(), negative));
        };

        // Confusingly, exponent is in base 2 for hex floating point numbers.
        let base: BigDecimal = 2.into();
        // Note: We cannot overflow/underflow BigDecimal here, as we will not be able to reach the
        // maximum/minimum scale (i64 range).
        let pow2 = pow_with_context(&base, exponent, &Context::default());

        bd * pow2
    } else {
        // scale != 0, which means that integral_only is not set, so only base 10 and 16 are allowed.
        unreachable!();
    };
    Ok(ExtendedBigDecimal::BigDecimal(bd))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ParseTarget {
    Decimal,
    Integral,
    Duration,
}

pub(crate) fn parse(
    input: &str,
    target: ParseTarget,
    allowed_suffixes: &[(char, u32)],
) -> Result<ExtendedBigDecimal, ExtendedParserError<ExtendedBigDecimal>> {
    // Note: literals with ' and " prefixes are parsed earlier on in argument parsing,
    // before UTF-8 conversion.

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
        } else if target == ParseTarget::Integral {
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

    // We only parse fractional and exponent part of the number in base 10/16 floating point numbers.
    let parse_frac_exp =
        matches!(base, Base::Decimal | Base::Hexadecimal) && target != ParseTarget::Integral;

    // Parse the integral and fractional (if supported) part of the number
    let (digits, scale, rest) = parse_digits(base, rest, parse_frac_exp);

    // Parse exponent part of the number for supported bases.
    let (exponent, rest) = if parse_frac_exp {
        parse_exponent(base, rest)
    } else {
        (None, rest)
    };

    // If no digit has been parsed, check if this is a special value, or declare the parsing unsuccessful
    if digits.is_none() {
        // If we trimmed an initial `0x`/`0b`, return a partial match.
        if let Some(partial) = unsigned.strip_prefix("0") {
            let ebd = if negative {
                ExtendedBigDecimal::MinusZero
            } else {
                ExtendedBigDecimal::zero()
            };
            return Err(ExtendedParserError::PartialMatch(ebd, partial.to_string()));
        }

        return if target == ParseTarget::Integral {
            Err(ExtendedParserError::NotNumeric)
        } else {
            parse_special_value(unsigned, negative, allowed_suffixes)
        };
    }

    let (mul, rest) = parse_suffix_multiplier(rest, allowed_suffixes);

    let digits = digits.unwrap() * mul;

    let ebd_result =
        construct_extended_big_decimal(digits, negative, base, scale, exponent.unwrap_or_default());

    // Return what has been parsed so far. If there are extra characters, mark the
    // parsing as a partial match.
    if rest.is_empty() {
        ebd_result
    } else {
        Err(ExtendedParserError::PartialMatch(
            ebd_result.unwrap_or_else(|e| e.extract()),
            rest.to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use bigdecimal::BigDecimal;

    use crate::extendedbigdecimal::ExtendedBigDecimal;

    use super::{ExtendedParser, ExtendedParserError};

    #[test]
    fn test_decimal_u64() {
        assert_eq!(Ok(123), u64::extended_parse("123"));
        assert_eq!(Ok(u64::MAX), u64::extended_parse(&format!("{}", u64::MAX)));
        assert_eq!(Ok(0), u64::extended_parse("-0"));
        assert_eq!(Ok(u64::MAX), u64::extended_parse("-1"));
        assert_eq!(
            Ok(u64::MAX / 2 + 1),
            u64::extended_parse("-9223372036854775808") // i64::MIN
        );
        assert_eq!(
            Ok(1123372036854675616),
            u64::extended_parse("-17323372036854876000") // 2*i64::MIN
        );
        assert_eq!(Ok(1), u64::extended_parse("-18446744073709551615")); // -u64::MAX
        assert!(matches!(
            u64::extended_parse("-18446744073709551616"), // -u64::MAX - 1
            Err(ExtendedParserError::Overflow(u64::MAX))
        ));
        assert!(matches!(
            u64::extended_parse("-92233720368547758150"),
            Err(ExtendedParserError::Overflow(u64::MAX))
        ));
        assert!(matches!(
            u64::extended_parse("-170141183460469231731687303715884105729"),
            Err(ExtendedParserError::Overflow(u64::MAX))
        ));
        assert!(matches!(
            u64::extended_parse(""),
            Err(ExtendedParserError::NotNumeric)
        ));
        assert_eq!(
            u64::extended_parse("123.15"),
            Err(ExtendedParserError::PartialMatch(123, ".15".to_string()))
        );
        assert_eq!(
            u64::extended_parse("123e10"),
            Err(ExtendedParserError::PartialMatch(123, "e10".to_string()))
        );
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
        assert_eq!(
            i64::extended_parse(&format!("{}", u64::MAX)),
            Err(ExtendedParserError::Overflow(i64::MAX))
        );
        assert!(matches!(
            i64::extended_parse(&format!("{}", i64::MAX as u64 + 1)),
            Err(ExtendedParserError::Overflow(i64::MAX))
        ));
        assert_eq!(
            i64::extended_parse("-123e10"),
            Err(ExtendedParserError::PartialMatch(-123, "e10".to_string()))
        );
        assert!(matches!(
            i64::extended_parse(&format!("{}", -(u64::MAX as i128))),
            Err(ExtendedParserError::Overflow(i64::MIN))
        ));
        assert!(matches!(
            i64::extended_parse(&format!("{}", i64::MIN as i128 - 1)),
            Err(ExtendedParserError::Overflow(i64::MIN))
        ));

        assert!(matches!(
            i64::extended_parse(""),
            Err(ExtendedParserError::NotNumeric)
        ));
        assert!(matches!(
            i64::extended_parse("."),
            Err(ExtendedParserError::NotNumeric)
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
        assert_eq!(Ok(12315000.0), f64::extended_parse("123.15e5"));
        assert_eq!(Ok(-12315000.0), f64::extended_parse("-123.15e5"));
        assert_eq!(Ok(12315000.0), f64::extended_parse("123.15E+5"));
        assert_eq!(Ok(0.0012315), f64::extended_parse("123.15E-5"));
        assert_eq!(
            Ok(0.15),
            f64::extended_parse(".150000000000000000000000000231313")
        );
        assert_eq!(
            f64::extended_parse("123.15e"),
            Err(ExtendedParserError::PartialMatch(123.15, "e".to_string()))
        );
        assert_eq!(
            f64::extended_parse("123.15E"),
            Err(ExtendedParserError::PartialMatch(123.15, "E".to_string()))
        );
        assert_eq!(
            f64::extended_parse("123.15e-"),
            Err(ExtendedParserError::PartialMatch(123.15, "e-".to_string()))
        );
        assert_eq!(
            f64::extended_parse("123.15e+"),
            Err(ExtendedParserError::PartialMatch(123.15, "e+".to_string()))
        );
        assert_eq!(
            f64::extended_parse("123.15e."),
            Err(ExtendedParserError::PartialMatch(123.15, "e.".to_string()))
        );
        assert_eq!(
            f64::extended_parse("1.2.3"),
            Err(ExtendedParserError::PartialMatch(1.2, ".3".to_string()))
        );
        assert_eq!(
            f64::extended_parse("123.15p5"),
            Err(ExtendedParserError::PartialMatch(123.15, "p5".to_string()))
        );
        // Minus zero. 0.0 == -0.0 so we explicitly check the sign.
        assert_eq!(Ok(0.0), f64::extended_parse("-0.0"));
        assert!(f64::extended_parse("-0.0").unwrap().is_sign_negative());
        assert_eq!(Ok(f64::INFINITY), f64::extended_parse("inf"));
        assert_eq!(Ok(f64::INFINITY), f64::extended_parse("+inf"));
        assert_eq!(Ok(f64::NEG_INFINITY), f64::extended_parse("-inf"));
        assert_eq!(Ok(f64::INFINITY), f64::extended_parse("Inf"));
        assert_eq!(Ok(f64::INFINITY), f64::extended_parse("InF"));
        assert_eq!(Ok(f64::INFINITY), f64::extended_parse("INF"));
        assert_eq!(Ok(f64::INFINITY), f64::extended_parse("infinity"));
        assert_eq!(Ok(f64::INFINITY), f64::extended_parse("+infiNIty"));
        assert_eq!(Ok(f64::NEG_INFINITY), f64::extended_parse("-INfinity"));
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
        assert_eq!(
            f64::extended_parse("-infinit"),
            Err(ExtendedParserError::PartialMatch(
                f64::NEG_INFINITY,
                "init".to_string()
            ))
        );
        assert_eq!(
            f64::extended_parse("-infinity00"),
            Err(ExtendedParserError::PartialMatch(
                f64::NEG_INFINITY,
                "00".to_string()
            ))
        );
        assert!(f64::extended_parse(&format!("{}", u64::MAX)).is_ok());
        assert!(f64::extended_parse(&format!("{}", i64::MIN)).is_ok());

        // f64 overflow/underflow
        assert!(matches!(
            f64::extended_parse("1.0e9000"),
            Err(ExtendedParserError::Overflow(f64::INFINITY))
        ));
        assert!(matches!(
            f64::extended_parse("-10.0e9000"),
            Err(ExtendedParserError::Overflow(f64::NEG_INFINITY))
        ));
        assert!(matches!(
            f64::extended_parse("1.0e-9000"),
            Err(ExtendedParserError::Underflow(0.0))
        ));
        assert!(matches!(
            f64::extended_parse("-1.0e-9000"),
            Err(ExtendedParserError::Underflow(f)) if f == 0.0 && f.is_sign_negative()));
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
        assert_eq!(
            Ok(ExtendedBigDecimal::BigDecimal(BigDecimal::from_bigint(
                12315.into(),
                -98
            ))),
            ExtendedBigDecimal::extended_parse("123.15e100")
        );
        assert_eq!(
            Ok(ExtendedBigDecimal::BigDecimal(BigDecimal::from_bigint(
                12315.into(),
                102
            ))),
            ExtendedBigDecimal::extended_parse("123.15E-100")
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

        // ExtendedBigDecimal overflow/underflow
        assert!(matches!(
            ExtendedBigDecimal::extended_parse(&format!("1e{}", i64::MAX as u64 + 2)),
            Err(ExtendedParserError::Overflow(ExtendedBigDecimal::Infinity))
        ));
        assert!(matches!(
            ExtendedBigDecimal::extended_parse(&format!("-0.1e{}", i64::MAX as u64 + 3)),
            Err(ExtendedParserError::Overflow(
                ExtendedBigDecimal::MinusInfinity
            ))
        ));
        assert!(matches!(
            ExtendedBigDecimal::extended_parse(&format!("1e{}", i64::MIN)),
            Err(ExtendedParserError::Underflow(ebd)) if ebd == ExtendedBigDecimal::zero()
        ));
        assert!(matches!(
            ExtendedBigDecimal::extended_parse(&format!("-0.01e{}", i64::MIN + 2)),
            Err(ExtendedParserError::Underflow(
                ExtendedBigDecimal::MinusZero
            ))
        ));

        // But no Overflow/Underflow if the digits are 0.
        assert_eq!(
            ExtendedBigDecimal::extended_parse(&format!("0e{}", i64::MAX as u64 + 2)),
            Ok(ExtendedBigDecimal::zero()),
        );
        assert_eq!(
            ExtendedBigDecimal::extended_parse(&format!("-0.0e{}", i64::MAX as u64 + 3)),
            Ok(ExtendedBigDecimal::MinusZero)
        );
        assert_eq!(
            ExtendedBigDecimal::extended_parse(&format!("0.0000e{}", i64::MIN)),
            Ok(ExtendedBigDecimal::zero()),
        );
        assert_eq!(
            ExtendedBigDecimal::extended_parse(&format!("-0e{}", i64::MIN + 2)),
            Ok(ExtendedBigDecimal::MinusZero)
        );

        /* Invalid numbers */
        assert_eq!(
            Err(ExtendedParserError::NotNumeric),
            ExtendedBigDecimal::extended_parse("")
        );
        assert_eq!(
            Err(ExtendedParserError::NotNumeric),
            ExtendedBigDecimal::extended_parse(".")
        );
        assert_eq!(
            Err(ExtendedParserError::NotNumeric),
            ExtendedBigDecimal::extended_parse("e")
        );
        assert_eq!(
            Err(ExtendedParserError::NotNumeric),
            ExtendedBigDecimal::extended_parse(".e")
        );
        assert_eq!(
            Err(ExtendedParserError::NotNumeric),
            ExtendedBigDecimal::extended_parse("-e")
        );
        assert_eq!(
            Err(ExtendedParserError::NotNumeric),
            ExtendedBigDecimal::extended_parse("+.e")
        );
        assert_eq!(
            Err(ExtendedParserError::NotNumeric),
            ExtendedBigDecimal::extended_parse("e10")
        );
        assert_eq!(
            Err(ExtendedParserError::NotNumeric),
            ExtendedBigDecimal::extended_parse("e-10")
        );
        assert_eq!(
            Err(ExtendedParserError::NotNumeric),
            ExtendedBigDecimal::extended_parse("-e10")
        );
        assert_eq!(
            Err(ExtendedParserError::NotNumeric),
            ExtendedBigDecimal::extended_parse("+e10")
        );
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
        assert_eq!(Ok(16.0), f64::extended_parse("0x0.8p5"));
        assert_eq!(Ok(0.0625), f64::extended_parse("0x1P-4"));

        // We cannot really check that 'e' is not a valid exponent indicator for hex floats...
        // but we can check that the number still gets parsed properly: 0x0.8e5 is 0x8e5 / 16**3
        assert_eq!(Ok(0.555908203125), f64::extended_parse("0x0.8e5"));

        assert_eq!(
            f64::extended_parse("0x0.1p"),
            Err(ExtendedParserError::PartialMatch(0.0625, "p".to_string()))
        );
        assert_eq!(
            f64::extended_parse("0x0.1p-"),
            Err(ExtendedParserError::PartialMatch(0.0625, "p-".to_string()))
        );
        assert_eq!(
            f64::extended_parse("0x.1p+"),
            Err(ExtendedParserError::PartialMatch(0.0625, "p+".to_string()))
        );
        assert_eq!(
            f64::extended_parse("0x.1p."),
            Err(ExtendedParserError::PartialMatch(0.0625, "p.".to_string()))
        );

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

        // Test very large exponents (they used to take forever as we kept all digits in the past)
        // Wolfram Alpha can get us (close to?) these values with a bit of log trickery:
        // 2**3000000000 = 10**log_10(2**3000000000) = 10**(3000000000 * log_10(2))
        // TODO: We do lose a little bit of precision, and the last digits are not be correct.
        assert_eq!(
            Ok(ExtendedBigDecimal::BigDecimal(
                // Wolfram Alpha says 9.8162042336235053508313854078782835648991393286913072670026492205522618203568834202759669215027003865... × 10^903089986
                BigDecimal::from_str("9.816204233623505350831385407878283564899139328691307267002649220552261820356883420275966921502700387e+903089986").unwrap()
            )),
            ExtendedBigDecimal::extended_parse("0x1p3000000000")
        );
        assert_eq!(
            Ok(ExtendedBigDecimal::BigDecimal(
                // Wolfram Alpha says 1.3492131462369983551036088935544888715959511045742395978049631768570509541390540646442193112226520316... × 10^-9030900
                BigDecimal::from_str("1.349213146236998355103608893554488871595951104574239597804963176857050954139054064644219311222652032e-9030900").unwrap()
            )),
            // Couldn't get a answer from Wolfram Alpha for smaller negative exponents
            ExtendedBigDecimal::extended_parse("0x1p-30000000")
        );

        // ExtendedBigDecimal overflow/underflow
        assert!(matches!(
            ExtendedBigDecimal::extended_parse(&format!("0x1p{}", u64::MAX as u128 + 1)),
            Err(ExtendedParserError::Overflow(ExtendedBigDecimal::Infinity))
        ));
        assert!(matches!(
            ExtendedBigDecimal::extended_parse(&format!("-0x100P{}", u64::MAX as u128 + 1)),
            Err(ExtendedParserError::Overflow(
                ExtendedBigDecimal::MinusInfinity
            ))
        ));
        assert!(matches!(
            ExtendedBigDecimal::extended_parse(&format!("0x1p-{}", u64::MAX as u128 + 1)),
            Err(ExtendedParserError::Underflow(ebd)) if ebd == ExtendedBigDecimal::zero()
        ));
        assert!(matches!(
            ExtendedBigDecimal::extended_parse(&format!("-0x0.100p-{}", u64::MAX as u128 + 1)),
            Err(ExtendedParserError::Underflow(
                ExtendedBigDecimal::MinusZero
            ))
        ));

        // Not actually hex numbers, but the prefixes look like it.
        assert_eq!(
            f64::extended_parse("0x"),
            Err(ExtendedParserError::PartialMatch(0.0, "x".to_string()))
        );
        assert_eq!(
            f64::extended_parse("0x."),
            Err(ExtendedParserError::PartialMatch(0.0, "x.".to_string()))
        );
        assert_eq!(
            f64::extended_parse("0xp"),
            Err(ExtendedParserError::PartialMatch(0.0, "xp".to_string()))
        );
        assert_eq!(
            f64::extended_parse("0xp-2"),
            Err(ExtendedParserError::PartialMatch(0.0, "xp-2".to_string()))
        );
        assert_eq!(
            f64::extended_parse("0x.p-2"),
            Err(ExtendedParserError::PartialMatch(0.0, "x.p-2".to_string()))
        );
        assert_eq!(
            f64::extended_parse("0X"),
            Err(ExtendedParserError::PartialMatch(0.0, "X".to_string()))
        );
        assert_eq!(
            f64::extended_parse("-0x"),
            Err(ExtendedParserError::PartialMatch(0.0, "x".to_string()))
        );
        assert_eq!(
            f64::extended_parse("+0x"),
            Err(ExtendedParserError::PartialMatch(0.0, "x".to_string()))
        );
        assert_eq!(
            f64::extended_parse("-0x."),
            Err(ExtendedParserError::PartialMatch(-0.0, "x.".to_string()))
        );
        assert_eq!(
            u64::extended_parse("0x"),
            Err(ExtendedParserError::PartialMatch(0, "x".to_string()))
        );
        assert_eq!(
            u64::extended_parse("-0x"),
            Err(ExtendedParserError::PartialMatch(0, "x".to_string()))
        );
        assert_eq!(
            i64::extended_parse("0x"),
            Err(ExtendedParserError::PartialMatch(0, "x".to_string()))
        );
        assert_eq!(
            i64::extended_parse("-0x"),
            Err(ExtendedParserError::PartialMatch(0, "x".to_string()))
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
        assert_eq!(
            u64::extended_parse("008"),
            Err(ExtendedParserError::PartialMatch(0, "8".to_string()))
        );
        assert_eq!(
            u64::extended_parse("08"),
            Err(ExtendedParserError::PartialMatch(0, "8".to_string()))
        );
        assert_eq!(
            u64::extended_parse("0."),
            Err(ExtendedParserError::PartialMatch(0, ".".to_string()))
        );

        // No float tests, leading zeros get parsed as decimal anyway.
    }

    #[test]
    fn test_binary() {
        assert_eq!(Ok(0b1011), u64::extended_parse("0b1011"));
        assert_eq!(Ok(0b1011), u64::extended_parse("0B1011"));
        assert_eq!(Ok(0b1011), u64::extended_parse("+0b1011"));
        assert_eq!(Ok(-0b1011), i64::extended_parse("-0b1011"));

        assert_eq!(
            u64::extended_parse("0b"),
            Err(ExtendedParserError::PartialMatch(0, "b".to_string()))
        );
        assert_eq!(
            u64::extended_parse("0b."),
            Err(ExtendedParserError::PartialMatch(0, "b.".to_string()))
        );
        assert_eq!(
            u64::extended_parse("-0b"),
            Err(ExtendedParserError::PartialMatch(0, "b".to_string()))
        );
        assert_eq!(
            i64::extended_parse("0b"),
            Err(ExtendedParserError::PartialMatch(0, "b".to_string()))
        );
        assert_eq!(
            i64::extended_parse("-0b"),
            Err(ExtendedParserError::PartialMatch(0, "b".to_string()))
        );

        // Binary not allowed for floats
        assert_eq!(
            f64::extended_parse("0b100"),
            Err(ExtendedParserError::PartialMatch(0f64, "b100".to_string()))
        );
        assert_eq!(
            f64::extended_parse("0b100.1"),
            Err(ExtendedParserError::PartialMatch(
                0f64,
                "b100.1".to_string()
            ))
        );

        assert_eq!(
            ExtendedBigDecimal::extended_parse("0b100.1"),
            Err(ExtendedParserError::PartialMatch(
                ExtendedBigDecimal::zero(),
                "b100.1".to_string()
            ))
        );

        assert_eq!(
            ExtendedBigDecimal::extended_parse("0b"),
            Err(ExtendedParserError::PartialMatch(
                ExtendedBigDecimal::zero(),
                "b".to_string()
            ))
        );
        assert_eq!(
            ExtendedBigDecimal::extended_parse("0b."),
            Err(ExtendedParserError::PartialMatch(
                ExtendedBigDecimal::zero(),
                "b.".to_string()
            ))
        );
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
            Err(ExtendedParserError::PartialMatch(6, " ".to_string())),
            u64::extended_parse("0x6 ")
        );
        assert_eq!(
            Err(ExtendedParserError::PartialMatch(7, "\t".to_string())),
            u64::extended_parse("0x7\t")
        );
        assert_eq!(
            Err(ExtendedParserError::PartialMatch(8, "\n".to_string())),
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
