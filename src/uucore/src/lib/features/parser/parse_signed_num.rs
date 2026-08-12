// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Parser for signed numeric arguments used by head, tail, and similar utilities.
//!
//! These utilities accept arguments like `-5`, `+10`, `-100K` where the leading
//! sign indicates different behavior (e.g., "first N" vs "last N" vs "starting from N").

use super::parse_size::{ParseSizeError, parse_size_u64, parse_size_u64_max, size_offset};

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
    let had_leading_zeros = trimmed.len() != size_string.len();
    let value = if trimmed.is_empty() {
        // All zeros (e.g., "000" or "0")
        0
    } else if had_leading_zeros && !trimmed.chars().any(|c| c.is_ascii_digit()) {
        // Only a size suffix remains after stripping leading zeros
        // (e.g., "0K"). Parse it as 1 unit to validate the suffix, but
        // the value is 0: a zero count times any unit is zero bytes.
        // Otherwise "0K" would parse as 1KiB (bare suffix means 1).
        // A genuinely bare suffix with no digits at all (e.g. "kiB")
        // still parses as 1 of that unit.
        parse_size_u64_max(trimmed)?;
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

/// Where the number starts inside `src`, past what this module strips before
/// parsing: the surrounding whitespace and the sign prefix.
///
/// The prefix here is the sign this module's own `strip_sign_prefix` takes, so
/// the two agree on what was stripped; see [`size_offset`] for the rest of the
/// reasoning.
///
/// # Arguments
///
/// * `src` - The argument as typed, the one [`parse_signed_num_max`] was given.
pub fn number_offset(src: &str) -> usize {
    size_offset(src, |c| matches!(c, '+' | '-'))
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

    /// The offset a caret counts has to be the one the parser skipped, or the
    /// underline lands beside what went wrong — under the sign rather than
    /// under the unit that was not a unit.
    #[test]
    fn number_offset_counts_what_the_parser_stripped() {
        assert_eq!(number_offset("1fb"), 0);
        assert_eq!(number_offset("-1fb"), 1);
        assert_eq!(number_offset("+5zz"), 1);
        assert_eq!(number_offset("  -1fb"), 3);
        // The parser reads leading zeros as part of the number, so they stay.
        assert_eq!(number_offset("-001fb"), 1);
    }

    /// What the two agree on is the point: `span` is only right about an
    /// operand once the sign in front of it has been counted out.
    #[test]
    fn the_span_lands_on_the_unit_of_a_signed_operand() {
        let operand = "-1fb";
        let error = parse_signed_num_max(operand).unwrap_err();
        let at = number_offset(operand);
        assert_eq!(error.span(&operand[at..]), 1..3);
        assert_eq!(&operand[at..][1..3], "fb");
    }

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
    fn test_zero_with_suffix() {
        // "0K" must be 0 bytes, not 1KiB (bare suffix parses as 1)
        let result = parse_signed_num_max("0K").unwrap();
        assert_eq!(result.value, 0);
        assert!(result.is_zero());

        let result = parse_signed_num_max("+0K").unwrap();
        assert_eq!(result.value, 0);
        assert!(result.has_plus());

        let result = parse_signed_num_max("-0M").unwrap();
        assert_eq!(result.value, 0);
        assert!(result.has_minus());

        let result = parse_signed_num_max("00K").unwrap();
        assert_eq!(result.value, 0);

        let result = parse_signed_num_max("0b").unwrap();
        assert_eq!(result.value, 0);
    }

    #[test]
    fn test_bare_suffix_still_parses_as_one() {
        // a bare suffix with no digits is 1 of that unit, not 0
        let result = parse_signed_num_max("kiB").unwrap();
        assert_eq!(result.value, 1024);

        let result = parse_signed_num_max("K").unwrap();
        assert_eq!(result.value, 1024);
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
