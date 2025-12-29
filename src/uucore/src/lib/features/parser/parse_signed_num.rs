// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Parser for signed numeric arguments used by head, tail, and similar utilities.
//!
//! These utilities accept arguments like `-5`, `+10`, `-100K` where the leading
//! sign indicates different behavior (e.g., "first N" vs "last N" vs "starting from N").

use super::parse_size::{ParseSizeError, parse_size_u64, parse_size_u64_max};

/// The sign prefix found on a numeric argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignPrefix {
    /// Plus sign prefix (e.g., "+10")
    Plus,
    /// Minus sign prefix (e.g., "-10")
    Minus,
}

/// A parsed signed numeric argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignedNum {
    /// The numeric value
    pub value: u64,
    /// The sign prefix that was present, if any
    pub sign: Option<SignPrefix>,
}

impl SignedNum {
    /// Returns true if the value is zero.
    pub fn is_zero(&self) -> bool {
        self.value == 0
    }

    /// Returns true if a plus sign was present.
    pub fn has_plus(&self) -> bool {
        self.sign == Some(SignPrefix::Plus)
    }

    /// Returns true if a minus sign was present.
    pub fn has_minus(&self) -> bool {
        self.sign == Some(SignPrefix::Minus)
    }
}

/// Parse a signed numeric argument, clamping to u64::MAX on overflow.
///
/// This function parses strings like "10", "+5K", "-100M" where:
/// - The optional leading `+` or `-` indicates direction/behavior
/// - The number can have size suffixes (K, M, G, etc.)
///
/// # Arguments
/// * `src` - The string to parse
///
/// # Returns
/// * `Ok(SignedNum)` - The parsed value and sign
/// * `Err(ParseSizeError)` - If the string cannot be parsed
///
/// # Examples
/// ```ignore
/// use uucore::parser::parse_signed_num::parse_signed_num_max;
///
/// let result = parse_signed_num_max("10").unwrap();
/// assert_eq!(result.value, 10);
/// assert_eq!(result.sign, None);
///
/// let result = parse_signed_num_max("+5K").unwrap();
/// assert_eq!(result.value, 5 * 1024);
/// assert_eq!(result.sign, Some(SignPrefix::Plus));
///
/// let result = parse_signed_num_max("-100").unwrap();
/// assert_eq!(result.value, 100);
/// assert_eq!(result.sign, Some(SignPrefix::Minus));
/// ```
pub fn parse_signed_num_max(src: &str) -> Result<SignedNum, ParseSizeError> {
    let (sign, size_string) = strip_sign_prefix(src);

    // Empty string after stripping sign is an error
    if size_string.is_empty() {
        return Err(ParseSizeError::ParseFailure(src.to_string()));
    }

    // Remove leading zeros so size is interpreted as decimal, not octal
    let trimmed = size_string.trim_start_matches('0');
    let value = if trimmed.is_empty() {
        // All zeros (e.g., "000" or "0")
        0
    } else {
        parse_size_u64_max(trimmed)?
    };

    Ok(SignedNum { value, sign })
}

/// Parse a signed numeric argument, returning error on overflow.
///
/// Same as [`parse_signed_num_max`] but returns an error instead of clamping
/// when the value overflows u64.
///
/// Note: On parse failure, this returns an error with the raw string (without quotes)
/// to allow callers to format the error message as needed.
pub fn parse_signed_num(src: &str) -> Result<SignedNum, ParseSizeError> {
    let (sign, size_string) = strip_sign_prefix(src);

    // Empty string after stripping sign is an error
    if size_string.is_empty() {
        return Err(ParseSizeError::ParseFailure(src.to_string()));
    }

    // Use parse_size_u64 but on failure, create our own error with the raw string
    // (without quotes) so callers can format it as needed
    let value = parse_size_u64(size_string)
        .map_err(|_| ParseSizeError::ParseFailure(size_string.to_string()))?;

    Ok(SignedNum { value, sign })
}

/// Strip the sign prefix from a string and return both the sign and remaining string.
fn strip_sign_prefix(src: &str) -> (Option<SignPrefix>, &str) {
    let trimmed = src.trim();

    if let Some(rest) = trimmed.strip_prefix('+') {
        (Some(SignPrefix::Plus), rest)
    } else if let Some(rest) = trimmed.strip_prefix('-') {
        (Some(SignPrefix::Minus), rest)
    } else {
        (None, trimmed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_sign() {
        let result = parse_signed_num_max("10").unwrap();
        assert_eq!(result.value, 10);
        assert_eq!(result.sign, None);
        assert!(!result.has_plus());
        assert!(!result.has_minus());
    }

    #[test]
    fn test_plus_sign() {
        let result = parse_signed_num_max("+10").unwrap();
        assert_eq!(result.value, 10);
        assert_eq!(result.sign, Some(SignPrefix::Plus));
        assert!(result.has_plus());
        assert!(!result.has_minus());
    }

    #[test]
    fn test_minus_sign() {
        let result = parse_signed_num_max("-10").unwrap();
        assert_eq!(result.value, 10);
        assert_eq!(result.sign, Some(SignPrefix::Minus));
        assert!(!result.has_plus());
        assert!(result.has_minus());
    }

    #[test]
    fn test_with_suffix() {
        let result = parse_signed_num_max("+5K").unwrap();
        assert_eq!(result.value, 5 * 1024);
        assert!(result.has_plus());

        let result = parse_signed_num_max("-2M").unwrap();
        assert_eq!(result.value, 2 * 1024 * 1024);
        assert!(result.has_minus());
    }

    #[test]
    fn test_zero() {
        let result = parse_signed_num_max("0").unwrap();
        assert_eq!(result.value, 0);
        assert!(result.is_zero());

        let result = parse_signed_num_max("+0").unwrap();
        assert_eq!(result.value, 0);
        assert!(result.is_zero());
        assert!(result.has_plus());

        let result = parse_signed_num_max("-0").unwrap();
        assert_eq!(result.value, 0);
        assert!(result.is_zero());
        assert!(result.has_minus());
    }

    #[test]
    fn test_leading_zeros() {
        let result = parse_signed_num_max("007").unwrap();
        assert_eq!(result.value, 7);

        let result = parse_signed_num_max("+007").unwrap();
        assert_eq!(result.value, 7);
        assert!(result.has_plus());

        let result = parse_signed_num_max("000").unwrap();
        assert_eq!(result.value, 0);
    }

    #[test]
    fn test_whitespace() {
        let result = parse_signed_num_max("  10  ").unwrap();
        assert_eq!(result.value, 10);

        let result = parse_signed_num_max("  +10  ").unwrap();
        assert_eq!(result.value, 10);
        assert!(result.has_plus());
    }

    #[test]
    fn test_overflow_max() {
        // Should clamp to u64::MAX instead of error
        let result = parse_signed_num_max("99999999999999999999999999").unwrap();
        assert_eq!(result.value, u64::MAX);
    }

    #[test]
    fn test_invalid() {
        assert!(parse_signed_num_max("").is_err());
        assert!(parse_signed_num_max("abc").is_err());
        assert!(parse_signed_num_max("++10").is_err());
    }
}
