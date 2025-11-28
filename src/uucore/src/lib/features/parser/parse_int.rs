// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Parser for integers with support for decimal, hexadecimal, and octal formats.

/// Parse an integer string with support for hex (0x/0X) and octal (0) prefixes.
///
/// Returns `None` if parsing fails or the value exceeds `u32::MAX`.
///
pub fn parse_u32_with_radix(arg: &str) -> Option<u32> {
    if let Some(hex) = arg.strip_prefix("0x").or_else(|| arg.strip_prefix("0X")) {
        u32::from_str_radix(hex, 16).ok()
    } else if let Some(octal) = arg.strip_prefix('0') {
        if octal.is_empty() {
            Some(0)
        } else {
            u32::from_str_radix(octal, 8).ok()
        }
    } else {
        arg.parse::<u32>().ok()
    }
}

/// Parse an integer string and wrap to u16 range.
///
/// Supports hex (0x/0X) and octal (0) prefixes. Values are wrapped using modulo arithmetic.
pub fn parse_u16_wrapped(arg: &str) -> Option<u16> {
    let n = parse_u32_with_radix(arg)?;
    Some((n % (u16::MAX as u32 + 1)) as u16)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_u32_decimal() {
        assert_eq!(parse_u32_with_radix("0"), Some(0));
        assert_eq!(parse_u32_with_radix("123"), Some(123));
        assert_eq!(parse_u32_with_radix("4294967295"), Some(u32::MAX));
    }

    #[test]
    fn test_parse_u32_hex() {
        assert_eq!(parse_u32_with_radix("0x0"), Some(0));
        assert_eq!(parse_u32_with_radix("0x1E"), Some(30));
        assert_eq!(parse_u32_with_radix("0X1E"), Some(30));
        assert_eq!(parse_u32_with_radix("0xFFFFFFFF"), Some(u32::MAX));
    }

    #[test]
    fn test_parse_u32_octal() {
        assert_eq!(parse_u32_with_radix("00"), Some(0));
        assert_eq!(parse_u32_with_radix("036"), Some(30));
        assert_eq!(parse_u32_with_radix("037777777777"), Some(u32::MAX));
    }

    #[test]
    fn test_parse_u32_invalid() {
        assert_eq!(parse_u32_with_radix(""), None);
        assert_eq!(parse_u32_with_radix("abc"), None);
        assert_eq!(parse_u32_with_radix("0xGGG"), None);
        assert_eq!(parse_u32_with_radix("4294967296"), None); // overflow
    }

    #[test]
    fn test_parse_u16_wrapped() {
        assert_eq!(parse_u16_wrapped("30"), Some(30));
        assert_eq!(parse_u16_wrapped("0x1E"), Some(30));
        assert_eq!(parse_u16_wrapped("036"), Some(30));
        assert_eq!(parse_u16_wrapped("65535"), Some(u16::MAX));
        assert_eq!(parse_u16_wrapped("65536"), Some(0));
        assert_eq!(parse_u16_wrapped("65537"), Some(1));
    }
}
