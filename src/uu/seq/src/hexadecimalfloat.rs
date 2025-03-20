// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore extendedbigdecimal bigdecimal hexdigit numberparse

// TODO: Rewrite this
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

/* TODO: move tests
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
*/
