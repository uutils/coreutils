// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore extendedbigdecimal bigdecimal hexdigit numberparse
use crate::number::PreciseNumber;
use crate::numberparse::ParseNumberError;
use bigdecimal::BigDecimal;
use num_traits::FromPrimitive;
use uucore::format::ExtendedBigDecimal;

/// The base of the hex number system
const HEX_RADIX: u32 = 16;

///  Parse a number from a floating-point hexadecimal exponent notation.
///
/// # Errors
/// Returns [`Err`] if:
/// - the input string is not a valid hexadecimal string
/// - the input data can't be interpreted as ['f64'] or ['BigDecimal']
///
/// # Examples
///
/// ```rust,ignore
/// let input = "0x1.4p-2";
/// let expected = 0.3125;
/// match input.parse_number::<PreciseNumber>().unwrap().number {
///     ExtendedBigDecimal::BigDecimal(bd)  => assert_eq!(bd.to_f64().unwrap(),expected),
///     _ => unreachable!()
/// };
/// ```
pub fn parse_number(s: &str) -> Result<PreciseNumber, ParseNumberError> {
    // Parse floating point parts
    let (sign, remain) = parse_sign_multiplier(s.trim())?;
    let remain = parse_hex_prefix(remain)?;
    let (integral_part, remain) = parse_integral_part(remain)?;
    let (fractional_part, remain) = parse_fractional_part(remain)?;
    let (exponent_part, remain) = parse_exponent_part(remain)?;

    // Check parts. Rise error if:
    // - The input string is not fully consumed
    // - Only integral part is presented
    // - Only exponent part is presented
    // - All 3 parts are empty
    match (
        integral_part,
        fractional_part,
        exponent_part,
        remain.is_empty(),
    ) {
        (_, _, _, false)
        | (Some(_), None, None, _)
        | (None, None, Some(_), _)
        | (None, None, None, _) => return Err(ParseNumberError::Float),
        _ => (),
    };

    // Build a number from parts
    let integral_value = integral_part.unwrap_or(0.0);
    let fractional_value = fractional_part.unwrap_or(0.0);
    let exponent_value = (2.0_f64).powi(exponent_part.unwrap_or(0));
    let value = sign * (integral_value + fractional_value) * exponent_value;

    // Build a PreciseNumber
    let number = BigDecimal::from_f64(value).ok_or(ParseNumberError::Float)?;
    let num_fractional_digits = number.fractional_digit_count().max(0) as u64;
    let num_integral_digits = if value.abs() < 1.0 {
        0
    } else {
        number.digits() - num_fractional_digits
    };
    let num_integral_digits = num_integral_digits + if sign < 0.0 { 1 } else { 0 };

    Ok(PreciseNumber::new(
        ExtendedBigDecimal::BigDecimal(number),
        num_integral_digits as usize,
        num_fractional_digits as usize,
    ))
}

// Detect number precision similar to GNU coreutils. Refer to scan_arg in seq.c. There are still
// some differences from the GNU version, but this should be sufficient to test the idea.
pub fn parse_precision(s: &str) -> Option<usize> {
    let hex_index = s.find(['x', 'X']);
    let point_index = s.find('.');

    if hex_index.is_some() {
        // Hex value. Returns:
        // - 0 for a hexadecimal integer (filled above)
        // - None for a hexadecimal floating-point number (the default value of precision)
        let power_index = s.find(['p', 'P']);
        if point_index.is_none() && power_index.is_none() {
            // No decimal point and no 'p' (power) => integer => precision = 0
            return Some(0);
        } else {
            return None;
        }
    }

    // This is a decimal floating point. The precision depends on two parameters:
    // - the number of fractional digits
    // - the exponent
    // Let's detect the number of fractional digits
    let fractional_length = if let Some(point_index) = point_index {
        s[point_index + 1..]
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .count()
    } else {
        0
    };

    let mut precision = Some(fractional_length);

    // Let's update the precision if exponent is present
    if let Some(exponent_index) = s.find(['e', 'E']) {
        let exponent_value: i32 = s[exponent_index + 1..].parse().unwrap_or(0);
        if exponent_value < 0 {
            precision = precision.map(|p| p + exponent_value.unsigned_abs() as usize);
        } else {
            precision = precision.map(|p| p - p.min(exponent_value as usize));
        }
    }
    precision
}

/// Parse the sign multiplier.
///
/// If a sign is present, the function reads and converts it into a multiplier.
/// If no sign is present, a multiplier of 1.0 is used.
///
/// # Errors
///
/// Returns [`Err`] if the input string does not start with a recognized sign or '0' symbol.
fn parse_sign_multiplier(s: &str) -> Result<(f64, &str), ParseNumberError> {
    if let Some(remain) = s.strip_prefix('-') {
        Ok((-1.0, remain))
    } else if let Some(remain) = s.strip_prefix('+') {
        Ok((1.0, remain))
    } else if s.starts_with('0') {
        Ok((1.0, s))
    } else {
        Err(ParseNumberError::Float)
    }
}

/// Parses the `0x` prefix in a case-insensitive manner.
///
/// # Errors
///
/// Returns [`Err`] if the input string does not contain the required prefix.
fn parse_hex_prefix(s: &str) -> Result<&str, ParseNumberError> {
    if !(s.starts_with("0x") || s.starts_with("0X")) {
        return Err(ParseNumberError::Float);
    }
    Ok(&s[2..])
}

/// Parse the integral part in hexadecimal notation.
///
/// The integral part is hexadecimal number located after the '0x' prefix and before '.' or 'p'
/// symbols. For example, the number 0x1.234p2 has an integral part 1.
///
/// This part is optional.
///
/// # Errors
///
/// Returns [`Err`] if the integral part is present but a hexadecimal number cannot be parsed from the input string.
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
/// This part is optional.
///
/// # Errors
///
/// Returns [`Err`] if the fractional part is present but a hexadecimal number cannot be parsed from the input string.
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

/// Parse the exponent part in hexadecimal notation.
///
/// The exponent part is a decimal number located after the 'p' symbol.
/// For example, the number 0x1.234p2 has an exponent part 2.
///
/// This part is optional.
///
/// # Errors
///
/// Returns [`Err`] if the exponent part is presented but a decimal number cannot be parsed from
/// the input string.
fn parse_exponent_part(s: &str) -> Result<(Option<i32>, &str), ParseNumberError> {
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

    use super::{parse_number, parse_precision};
    use crate::{ExtendedBigDecimal, numberparse::ParseNumberError};
    use bigdecimal::BigDecimal;
    use num_traits::ToPrimitive;

    fn parse_big_decimal(s: &str) -> Result<BigDecimal, ParseNumberError> {
        match parse_number(s)?.number {
            ExtendedBigDecimal::BigDecimal(bd) => Ok(bd),
            _ => Err(ParseNumberError::Float),
        }
    }

    fn parse_f64(s: &str) -> Result<f64, ParseNumberError> {
        parse_big_decimal(s)?
            .to_f64()
            .ok_or(ParseNumberError::Float)
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
        assert_eq!(parse_f64("-0x.1p-3").unwrap(), -0.0078125);
        assert_eq!(parse_f64("-0x.ep-3").unwrap(), -0.109375);
    }

    #[test]
    fn test_parse_float_from_invalid_values() {
        let expected_error = ParseNumberError::Float;
        assert_eq!(parse_f64("").unwrap_err(), expected_error);
        assert_eq!(parse_f64("1").unwrap_err(), expected_error);
        assert_eq!(parse_f64("1p").unwrap_err(), expected_error);
        assert_eq!(parse_f64("0x").unwrap_err(), expected_error);
        assert_eq!(parse_f64("0xG").unwrap_err(), expected_error);
        assert_eq!(parse_f64("0xp").unwrap_err(), expected_error);
        assert_eq!(parse_f64("0xp3").unwrap_err(), expected_error);
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
        assert_eq!(parse_f64("-0x.ep-3z").unwrap_err(), expected_error);
    }

    #[test]
    fn test_parse_precise_number_count_digits() {
        let precise_num = parse_number("0x1.2").unwrap(); // 1.125 decimal
        assert_eq!(precise_num.num_integral_digits, 1);
        assert_eq!(precise_num.num_fractional_digits, 3);

        let precise_num = parse_number("-0x1.2").unwrap(); // -1.125 decimal
        assert_eq!(precise_num.num_integral_digits, 2);
        assert_eq!(precise_num.num_fractional_digits, 3);

        let precise_num = parse_number("0x123.8").unwrap(); // 291.5 decimal
        assert_eq!(precise_num.num_integral_digits, 3);
        assert_eq!(precise_num.num_fractional_digits, 1);

        let precise_num = parse_number("-0x123.8").unwrap(); // -291.5 decimal
        assert_eq!(precise_num.num_integral_digits, 4);
        assert_eq!(precise_num.num_fractional_digits, 1);
    }

    #[test]
    fn test_parse_precision_valid_values() {
        assert_eq!(parse_precision("1"), Some(0));
        assert_eq!(parse_precision("0x1"), Some(0));
        assert_eq!(parse_precision("0x1.1"), None);
        assert_eq!(parse_precision("0x1.1p2"), None);
        assert_eq!(parse_precision("0x1.1p-2"), None);
        assert_eq!(parse_precision(".1"), Some(1));
        assert_eq!(parse_precision("1.1"), Some(1));
        assert_eq!(parse_precision("1.12"), Some(2));
        assert_eq!(parse_precision("1.12345678"), Some(8));
        assert_eq!(parse_precision("1.12345678e-3"), Some(11));
        assert_eq!(parse_precision("1.1e-1"), Some(2));
        assert_eq!(parse_precision("1.1e-3"), Some(4));
    }

    #[test]
    fn test_parse_precision_invalid_values() {
        // Just to make sure it doesn't crash on incomplete values/bad format
        // Good enough for now.
        assert_eq!(parse_precision("1."), Some(0));
        assert_eq!(parse_precision("1e"), Some(0));
        assert_eq!(parse_precision("1e-"), Some(0));
        assert_eq!(parse_precision("1e+"), Some(0));
        assert_eq!(parse_precision("1em"), Some(0));
    }
}
