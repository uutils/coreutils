// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore extendedbigdecimal bigdecimal hexdigit numberparse
use crate::extendedbigdecimal::ExtendedBigDecimal;
use crate::number::PreciseNumber;
use crate::numberparse::ParseNumberError;
use bigdecimal::BigDecimal;
use num_traits::FromPrimitive;

/// The base of the hex number system
const HEX_RADIX: u32 = 16;

///  Parse a number from a floating-point hexadecimal exponent notation.
///
/// # Errors
///
/// This function returns an error if:
/// - the input string is not a valid hexadecimal string
/// - the input data can't be interpreted as ['f64'] or ['BigDecimal']
///
/// # Examples
///
/// ```rust,ignore
/// let input = "0x1.4p-2";
/// let expected = 0.3125;
/// match input.parse::<PreciseNumber>().unwrap().number {
///     ExtendedBigDecimal::BigDecimal(bd)  => assert_eq!(bd.to_f64().unwrap(),expected),
///     _ => unreachable!()
/// };
/// ```
pub fn parse_hexadecimal_float(s: &str) -> Result<PreciseNumber, ParseNumberError> {
    // Parse floating point parts
    let (sign, remain) = parse_sign(s.trim())?;
    let remain = parse_hex_prefix(remain)?;
    let (integer, remain) = parse_integral_part(remain)?;
    let (fractional, remain) = parse_fractional_part(remain)?;
    let (power, remain) = parse_power_part(remain)?;

    // Check parts:
    // - Both 'fractional' and 'power' values cannot be 'None' at the same time.
    // - The entire string must be consumed; otherwise, there could be garbage symbols after the Hex float.
    if fractional.is_none() && power.is_none() {
        return Err(ParseNumberError::Float);
    }

    if !remain.is_empty() {
        return Err(ParseNumberError::Float);
    }

    // Build a number from parts
    let value = sign
        * (integer.unwrap_or(0.0) + fractional.unwrap_or(0.0))
        * (2.0_f64).powi(power.unwrap_or(0));

    // Build a PreciseNumber
    let number = BigDecimal::from_f64(value).ok_or(ParseNumberError::Float)?;
    let fractional_digits = i64::max(number.fractional_digit_count(), 0) as usize;
    Ok(PreciseNumber::new(
        ExtendedBigDecimal::BigDecimal(number),
        0,
        fractional_digits,
    ))
}

fn parse_sign(s: &str) -> Result<(f64, &str), ParseNumberError> {
    if let Some(remain) = s.strip_prefix('-') {
        Ok((-1.0, remain))
    } else if let Some(remain) = s.strip_prefix('+') {
        Ok((1.0, remain))
    } else {
        Ok((1.0, s))
    }
}

fn parse_hex_prefix(s: &str) -> Result<&str, ParseNumberError> {
    if !(s.starts_with("0x") || s.starts_with("0X")) {
        return Err(ParseNumberError::Float);
    }
    Ok(&s[2..])
}

fn parse_integral_part(s: &str) -> Result<(Option<f64>, &str), ParseNumberError> {
    // This part is optional. Skip parsing if symbol is not a hex digit.
    let length = s.chars().take_while(|c| c.is_ascii_hexdigit()).count();
    if length > 0 {
        let integer =
            u64::from_str_radix(&s[..length], HEX_RADIX).map_err(|_| ParseNumberError::Float)?;
        Ok((Some(integer as f64), &s[length..]))
    } else {
        Ok((None, s))
    }
}

/// Parse the fractional part in hexadecimal notation.
///
/// The function calculates the sum of the digits after the '.' (dot) sign. Each Nth digit is
/// interpreted as digit / 16^n, where n represents the position after the dot starting from 1.
///
/// For example, the number 0x1.234p2 has a fractional part 234, which can be interpreted as
/// 2/16^1 + 3/16^2 + 4/16^3, where 16 is the radix of the hexadecimal number system. This equals
/// 0.125 + 0.01171875 + 0.0009765625 = 0.1376953125 in decimal. And this is exactly what the
/// function does.
///
/// # Errors
///
/// This function returns an error if the string is empty or contains characters that are not hex
/// digits.
fn parse_fractional_part(s: &str) -> Result<(Option<f64>, &str), ParseNumberError> {
    // This part is optional and follows after the '.' symbol. Skip parsing if the dot is not present.
    if !s.starts_with('.') {
        return Ok((None, s));
    }

    let s = &s[1..];
    let mut multiplier = 1.0 / HEX_RADIX as f64;
    let mut total = 0.0;
    let mut length = 0;

    for c in s.chars().take_while(|c| c.is_ascii_hexdigit()) {
        let digit = c
            .to_digit(HEX_RADIX)
            .map(|x| x as u8)
            .ok_or(ParseNumberError::Float)?;
        total += (digit as f64) * multiplier;
        multiplier /= HEX_RADIX as f64;
        length += 1;
    }

    if length == 0 {
        return Err(ParseNumberError::Float);
    }
    Ok((Some(total), &s[length..]))
}

fn parse_power_part(s: &str) -> Result<(Option<i32>, &str), ParseNumberError> {
    // This part is optional and follows after 'p' or 'P' symbols. Skip parsing if the symbols are not present
    if !(s.starts_with('p') || s.starts_with('P')) {
        return Ok((None, s));
    }

    let s = &s[1..];
    let length = s
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '-' || *c == '+')
        .count();

    if length == 0 {
        return Err(ParseNumberError::Float);
    }

    let value = s[..length].parse().map_err(|_| ParseNumberError::Float)?;
    Ok((Some(value), &s[length..]))
}

#[cfg(test)]
mod tests {

    use super::parse_hexadecimal_float;
    use crate::{numberparse::ParseNumberError, ExtendedBigDecimal};
    use num_traits::ToPrimitive;

    fn parse_f64(s: &str) -> Result<f64, ParseNumberError> {
        match parse_hexadecimal_float(s)?.number {
            ExtendedBigDecimal::BigDecimal(bd) => bd.to_f64().ok_or(ParseNumberError::Float),
            _ => Err(ParseNumberError::Float),
        }
    }

    #[test]
    fn test_parse_precise_number_case_insensitive() {
        assert_eq!(parse_f64("0x1P1").unwrap(), 2.0);
        assert_eq!(parse_f64("0x1p1").unwrap(), 2.0);
    }

    #[test]
    fn test_parse_precise_number_plus_minus_prefixes() {
        assert_eq!(parse_f64("+0x1p1").unwrap(), 2.0);
        assert_eq!(parse_f64("-0x1p1").unwrap(), -2.0);
    }

    #[test]
    fn test_parse_precise_number_power_signs() {
        assert_eq!(parse_f64("0x1p1").unwrap(), 2.0);
        assert_eq!(parse_f64("0x1p+1").unwrap(), 2.0);
        assert_eq!(parse_f64("0x1p-1").unwrap(), 0.5);
    }

    #[test]
    fn test_parse_precise_number_hex() {
        assert_eq!(parse_f64("0xd.dp-1").unwrap(), 6.90625);
    }

    #[test]
    fn test_parse_precise_number_no_power() {
        assert_eq!(parse_f64("0x123.a").unwrap(), 291.625);
    }

    #[test]
    fn test_parse_precise_number_no_fractional() {
        assert_eq!(parse_f64("0x333p-4").unwrap(), 51.1875);
    }

    #[test]
    fn test_parse_precise_number_no_integral() {
        assert_eq!(parse_f64("0x.9").unwrap(), 0.5625);
        assert_eq!(parse_f64("0x.9p2").unwrap(), 2.25);
    }

    #[test]
    fn test_parse_precise_number_from_valid_values() {
        assert_eq!(parse_f64("0x1p1").unwrap(), 2.0);
        assert_eq!(parse_f64("+0x1p1").unwrap(), 2.0);
        assert_eq!(parse_f64("-0x1p1").unwrap(), -2.0);
        assert_eq!(parse_f64("0x1p-1").unwrap(), 0.5);
        assert_eq!(parse_f64("0x1.8").unwrap(), 1.5);
        assert_eq!(parse_f64("-0x1.8").unwrap(), -1.5);
        assert_eq!(parse_f64("0x1.8p2").unwrap(), 6.0);
        assert_eq!(parse_f64("0x1.8p+2").unwrap(), 6.0);
        assert_eq!(parse_f64("0x1.8p-2").unwrap(), 0.375);
        assert_eq!(parse_f64("0x.8").unwrap(), 0.5);
        assert_eq!(parse_f64("0x10p0").unwrap(), 16.0);
        assert_eq!(parse_f64("0x0.0").unwrap(), 0.0);
        assert_eq!(parse_f64("0x0p0").unwrap(), 0.0);
        assert_eq!(parse_f64("0x0.0p0").unwrap(), 0.0);
    }

    #[test]
    fn test_parse_float_from_invalid_values() {
        let expected_error = ParseNumberError::Float;
        assert_eq!(parse_f64("").unwrap_err(), expected_error);
        assert_eq!(parse_f64("1").unwrap_err(), expected_error);
        assert_eq!(parse_f64("1p").unwrap_err(), expected_error);
        assert_eq!(parse_f64("0x1").unwrap_err(), expected_error);
        assert_eq!(parse_f64("0x1.").unwrap_err(), expected_error);
        assert_eq!(parse_f64("0x1p").unwrap_err(), expected_error);
        assert_eq!(parse_f64("0x1p+").unwrap_err(), expected_error);
        assert_eq!(parse_f64("-0xx1p1").unwrap_err(), expected_error);
        assert_eq!(parse_f64("0x1.k").unwrap_err(), expected_error);
        assert_eq!(parse_f64("0x1").unwrap_err(), expected_error);
        assert_eq!(parse_f64("-0x1pa").unwrap_err(), expected_error);
        assert_eq!(parse_f64("0x1.1pk").unwrap_err(), expected_error);
        assert_eq!(parse_f64("0x1.8p2z").unwrap_err(), expected_error);
        assert_eq!(parse_f64("0x1p3.2").unwrap_err(), expected_error);
    }
}
