// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use crate::extendedbigdecimal::ExtendedBigDecimal;
use crate::number::PreciseNumber;
use crate::numberparse::ParseNumberError;
use bigdecimal::BigDecimal;
use num_traits::FromPrimitive;

pub fn parse_hexadecimal_float(s: &str) -> Result<PreciseNumber, ParseNumberError> {
    let value = parse_float(s)?;
    let number = BigDecimal::from_f64(value).ok_or(ParseNumberError::Float)?;
    let fractional_digits = i64::max(number.fractional_digit_count(), 0) as usize;
    Ok(PreciseNumber::new(
        ExtendedBigDecimal::BigDecimal(number),
        0,
        fractional_digits,
    ))
}

fn parse_float(s: &str) -> Result<f64, ParseNumberError> {
    let mut s = s.trim();

    // Detect a sign
    let sign = if s.starts_with('-') {
        s = &s[1..];
        -1.0
    } else if s.starts_with('+') {
        s = &s[1..];
        1.0
    } else {
        1.0
    };

    // Is HEX?
    if s.starts_with("0x") || s.starts_with("0X") {
        s = &s[2..];
    } else {
        return Err(ParseNumberError::Float);
    }

    // Read an integer part (if presented)
    let length = s.chars().take_while(|c| c.is_ascii_hexdigit()).count();
    let integer = u64::from_str_radix(&s[..length], 16).unwrap_or(0);
    s = &s[length..];

    // Read a fractional part (if presented)
    let fractional = if s.starts_with('.') {
        s = &s[1..];
        let length = s.chars().take_while(|c| c.is_ascii_hexdigit()).count();
        let value = parse_fractional_part(&s[..length])?;
        s = &s[length..];
        Some(value)
    } else {
        None
    };

    // Read a power (if presented)
    let power = if s.starts_with('p') || s.starts_with('P') {
        s = &s[1..];
        let length = s
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '-' || *c == '+')
            .count();
        let value = s[..length].parse().map_err(|_| ParseNumberError::Float)?;
        s = &s[length..];
        Some(value)
    } else {
        None
    };

    // Post checks:
    // - Both Fractions & Power values can't be none in the same time
    // - string should be consumed. Otherwise, it's possible to have garbage symbols after the HEX
    // float
    if fractional.is_none() && power.is_none() {
        return Err(ParseNumberError::Float);
    }

    if !s.is_empty() {
        return Err(ParseNumberError::Float);
    }

    // Build the result
    let total =
        sign * (integer as f64 + fractional.unwrap_or(0.0)) * (2.0_f64).powi(power.unwrap_or(0));
    Ok(total)
}

fn parse_fractional_part(s: &str) -> Result<f64, ParseNumberError> {
    if s.is_empty() {
        return Err(ParseNumberError::Float);
    }

    let mut multiplier = 1.0 / 16.0;
    let mut total = 0.0;
    for c in s.chars() {
        let digit = c
            .to_digit(16)
            .map(|x| x as u8)
            .ok_or(ParseNumberError::Float)?;
        total += (digit as f64) * multiplier;
        multiplier /= 16.0;
    }
    Ok(total)
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
