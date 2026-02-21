// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore strtime

//! GNU date format modifier support
//!
//! This module implements GNU-compatible format modifiers for date formatting.
//! These modifiers extend standard strftime format specifiers with optional
//! width and flag modifiers.
//!
//! ## Syntax
//!
//! Format: `%[flags][width]specifier`
//!
//! ### Flags
//! - `-`: Do not pad the field
//! - `_`: Pad with spaces instead of zeros
//! - `0`: Pad with zeros (default for numeric fields)
//! - `^`: Convert to uppercase
//! - `#`: Use opposite case (uppercase becomes lowercase and vice versa)
//! - `+`: Force display of sign (+ for positive, - for negative)
//!
//! ### Width
//! - One or more digits specifying minimum field width
//! - Field will be padded to this width using the padding character
//!
//! ### Examples
//! - `%10Y`: Year padded to 10 digits with zeros (0000001999)
//! - `%_10m`: Month padded to 10 digits with spaces (        06)
//! - `%-d`: Day without padding (1 instead of 01)
//! - `%^B`: Month name in uppercase (JUNE)
//! - `%+4C`: Century with sign, padded to 4 characters (+019)

use jiff::Zoned;
use jiff::fmt::strtime::{BrokenDownTime, Config, PosixCustom};
use regex::Regex;
use std::fmt;
use std::sync::OnceLock;

/// Error type for format modifier operations
#[derive(Debug)]
pub enum FormatError {
    /// Error from the underlying jiff library
    JiffError(jiff::Error),
    /// Custom error message
    Custom(String),
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::JiffError(e) => write!(f, "{e}"),
            Self::Custom(s) => write!(f, "{s}"),
        }
    }
}

impl From<jiff::Error> for FormatError {
    fn from(e: jiff::Error) -> Self {
        Self::JiffError(e)
    }
}

const ERR_FIELD_WIDTH_TOO_LARGE: &str = "field width too large";

fn width_too_large_error() -> FormatError {
    FormatError::Custom(ERR_FIELD_WIDTH_TOO_LARGE.to_string())
}

/// Regex to match format specifiers with optional modifiers
/// Pattern: % \[flags\] \[width\] specifier
/// Flags: -, _, 0, ^, #, +
/// Width: one or more digits
/// Specifier: any letter or special sequence like :z, ::z, :::z
fn format_spec_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"%([_0^#+-]*)(\d*)(:*[a-zA-Z])").unwrap())
}

/// Check if format string contains any GNU modifiers and format if present.
///
/// This function combines modifier detection and formatting in a single pass
/// for better performance. If no modifiers are found, returns None and the
/// caller should use standard formatting. If modifiers are found, returns
/// the formatted string.
pub fn format_with_modifiers_if_present(
    date: &Zoned,
    format_string: &str,
    config: &Config<PosixCustom>,
) -> Option<Result<String, FormatError>> {
    let re = format_spec_regex();

    // Quick check: does the string contain any modifiers?
    let has_modifiers = re.captures_iter(format_string).any(|cap| {
        let flags = cap.get(1).map_or("", |m| m.as_str());
        let width_str = cap.get(2).map_or("", |m| m.as_str());
        !flags.is_empty() || !width_str.is_empty()
    });

    if !has_modifiers {
        return None;
    }

    // If we have modifiers, format the string
    Some(format_with_modifiers(date, format_string, config))
}

/// Process a format string with GNU modifiers.
///
/// # Arguments
/// * `date` - The date to format
/// * `format_string` - Format string with GNU modifiers
/// * `config` - Strftime configuration
///
/// # Returns
/// Formatted string with modifiers applied
///
/// # Errors
/// Returns `FormatError` if formatting fails
fn format_with_modifiers(
    date: &Zoned,
    format_string: &str,
    config: &Config<PosixCustom>,
) -> Result<String, FormatError> {
    // First, replace %% with a placeholder to avoid matching it
    let placeholder = "\x00PERCENT\x00";
    let temp_format = format_string.replace("%%", placeholder);

    let re = format_spec_regex();
    let mut result = String::new();
    let mut last_end = 0;

    let broken_down = BrokenDownTime::from(date);

    for cap in re.captures_iter(&temp_format) {
        let whole_match = cap.get(0).unwrap();
        let flags = cap.get(1).map_or("", |m| m.as_str());
        let width_str = cap.get(2).map_or("", |m| m.as_str());
        let spec = cap.get(3).unwrap().as_str();

        // Add text before this match
        result.push_str(&temp_format[last_end..whole_match.start()]);

        // Format the base specifier first
        let base_format = format!("%{spec}");
        let formatted = broken_down.to_string_with_config(config, &base_format)?;

        // Check if this specifier has modifiers
        if !flags.is_empty() || !width_str.is_empty() {
            // Apply modifiers to the formatted value
            let width: usize = width_str.parse().unwrap_or(0);
            let modified = apply_modifiers(&formatted, flags, width, spec)?;
            result.push_str(&modified);
        } else {
            // No modifiers, use formatted value as-is
            result.push_str(&formatted);
        }

        last_end = whole_match.end();
    }

    // Add remaining text
    result.push_str(&temp_format[last_end..]);

    // Restore %% by converting placeholder to %
    let result = result.replace(placeholder, "%");

    Ok(result)
}

/// Returns true if the specifier produces text output (default pad is space)
/// rather than numeric output (default pad is zero).
fn is_text_specifier(specifier: &str) -> bool {
    matches!(
        specifier.chars().last(),
        Some('A' | 'a' | 'B' | 'b' | 'h' | 'Z' | 'p' | 'P')
    )
}

/// Strip default padding (leading zeros or leading spaces) from a value,
/// preserving at least one character.
fn strip_default_padding(value: &str) -> String {
    if value.starts_with('0') && value.len() >= 2 {
        let stripped = value.trim_start_matches('0');
        if stripped.is_empty() {
            return "0".to_string();
        }
        if let Some(first_char) = stripped.chars().next() {
            if first_char.is_ascii_digit() {
                return stripped.to_string();
            }
        }
    }
    if value.starts_with(' ') {
        let stripped = value.trim_start();
        if !stripped.is_empty() {
            return stripped.to_string();
        }
    }
    value.to_string()
}

/// Apply width and flag modifiers to a formatted value.
///
/// The `specifier` parameter is the format specifier (e.g., "d", "B", "Y")
/// which determines the default padding character (space for text, zero for numeric).
/// Flags are processed in order so that when conflicting flags appear,
/// the last one takes precedence (e.g., `_+` means `+` wins for padding).
fn apply_modifiers(
    value: &str,
    flags: &str,
    width: usize,
    specifier: &str,
) -> Result<String, FormatError> {
    let mut result = value.to_string();

    // Determine default pad character based on specifier type
    let default_pad = if is_text_specifier(specifier) {
        ' '
    } else {
        '0'
    };

    // Process flags in order - last conflicting flag wins
    let mut pad_char = default_pad;
    let mut no_pad = false;
    let mut uppercase = false;
    let mut swap_case = false;
    let mut force_sign = false;

    for flag in flags.chars() {
        match flag {
            '-' => {
                no_pad = true;
            }
            '_' => {
                no_pad = false;
                pad_char = ' ';
            }
            '0' => {
                no_pad = false;
                pad_char = '0';
            }
            '^' => {
                uppercase = true;
                swap_case = false; // ^ overrides #
            }
            '#' => {
                if !uppercase {
                    // Only apply # if ^ hasn't been set
                    swap_case = true;
                }
            }
            '+' => {
                force_sign = true;
                no_pad = false;
                pad_char = '0';
            }
            _ => {}
        }
    }

    // Apply case modifications (uppercase takes precedence over swap_case)
    if uppercase {
        result = result.to_uppercase();
    } else if swap_case {
        if result
            .chars()
            .all(|c| !c.is_alphabetic() || c.is_uppercase())
        {
            result = result.to_lowercase();
        } else {
            result = result.to_uppercase();
        }
    }

    // If no_pad flag is active, suppress all padding and return
    if no_pad {
        return Ok(strip_default_padding(&result));
    }

    // Handle width smaller than result: strip default padding to fit
    if width > 0 && width < result.len() {
        return Ok(strip_default_padding(&result));
    }

    // Strip leading zeros when switching to space padding on numeric fields
    if pad_char == ' '
        && !is_text_specifier(specifier)
        && result.starts_with('0')
        && result.len() >= 2
    {
        result = strip_default_padding(&result);
    }

    // Apply force sign for numeric values
    if force_sign && !result.starts_with('+') && !result.starts_with('-') {
        if result.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            result.insert(0, '+');
        }
    }

    // Apply width padding
    if width > result.len() {
        let padding = width - result.len();
        let has_sign = result.starts_with('+') || result.starts_with('-');

        if pad_char == '0' && has_sign {
            // Zero padding: sign first, then zeros (e.g., "-0022")
            let sign = result.chars().next().unwrap();
            let rest = &result[1..];
            let target_len = result
                .len()
                .checked_add(padding)
                .ok_or_else(width_too_large_error)?;
            let mut padded = String::new();
            padded
                .try_reserve(target_len)
                .map_err(|_| width_too_large_error())?;
            padded.push(sign);
            padded.extend(std::iter::repeat_n('0', padding));
            padded.push_str(rest);
            result = padded;
        } else {
            // Default: pad on the left (e.g., "  -22" or "  1999")
            let target_len = result
                .len()
                .checked_add(padding)
                .ok_or_else(width_too_large_error)?;
            let mut padded = String::new();
            padded
                .try_reserve(target_len)
                .map_err(|_| width_too_large_error())?;
            padded.extend(std::iter::repeat_n(pad_char, padding));
            padded.push_str(&result);
            result = padded;
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::{civil, tz::TimeZone};

    fn make_test_date(year: i16, month: i8, day: i8, hour: i8) -> Zoned {
        civil::date(year, month, day)
            .at(hour, 0, 0, 0)
            .to_zoned(TimeZone::UTC)
            .unwrap()
    }

    fn get_config() -> Config<PosixCustom> {
        Config::new().custom(PosixCustom::new()).lenient(true)
    }

    #[test]
    fn test_width_and_padding_modifiers() {
        let date = make_test_date(1999, 6, 1, 0);
        let config = get_config();

        // Test basic width with zero padding
        let result = format_with_modifiers(&date, "%10Y", &config).unwrap();
        assert_eq!(result, "0000001999");

        // Test large width
        let result = format_with_modifiers(&date, "%20Y", &config).unwrap();
        assert_eq!(result, "00000000000000001999");
        assert_eq!(result.len(), 20);

        // Test underscore (space) padding with month
        let result = format_with_modifiers(&date, "%_10m", &config).unwrap();
        assert_eq!(result, "         6");
        assert_eq!(result.len(), 10);

        // Test underscore padding with day
        let date_day5 = make_test_date(1999, 6, 5, 0);
        let result = format_with_modifiers(&date_day5, "%_10d", &config).unwrap();
        assert_eq!(result, "         5");
    }

    #[test]
    fn test_no_pad_and_case_flags() {
        let date = make_test_date(1999, 6, 1, 0);
        let config = get_config();

        // Test no-pad: %-10Y suppresses all padding (width ignored)
        let result = format_with_modifiers(&date, "%-10Y", &config).unwrap();
        assert_eq!(result, "1999");

        // Test no-pad: %-d strips default zero padding
        let result = format_with_modifiers(&date, "%-d", &config).unwrap();
        assert_eq!(result, "1");

        // Test uppercase: %^B should uppercase month name
        let result = format_with_modifiers(&date, "%^B", &config).unwrap();
        assert_eq!(result, "JUNE");

        // Test uppercase with width: %^10B should uppercase and space-pad (text specifier)
        let result = format_with_modifiers(&date, "%^10B", &config).unwrap();
        assert_eq!(result, "      JUNE");
        assert_eq!(result.len(), 10);
    }

    #[test]
    fn test_sign_flags() {
        let date = make_test_date(1970, 1, 1, 0);
        let config = get_config();

        // Test force sign with century: %+4C
        let result = format_with_modifiers(&date, "%+4C", &config).unwrap();
        assert!(result.starts_with('+'));
        assert_eq!(result.len(), 4);

        // Test force sign with zero padding: %+6Y
        let result = format_with_modifiers(&date, "%+6Y", &config).unwrap();
        assert_eq!(result, "+01970");
    }

    #[test]
    fn test_combined_flags_underscore_and_sign() {
        let date = make_test_date(1970, 1, 1, 0);
        let config = get_config();
        // %_+6Y: _ sets space pad, then + overrides to zero pad with sign (last wins)
        let result = format_with_modifiers(&date, "%_+6Y", &config).unwrap();
        assert_eq!(result, "+01970");
    }

    #[test]
    fn test_combined_flags_no_pad_and_uppercase() {
        let date = make_test_date(1999, 6, 1, 0);
        let config = get_config();
        // %-^10B: uppercase + no-pad (- suppresses all padding, width ignored)
        let result = format_with_modifiers(&date, "%-^10B", &config).unwrap();
        assert_eq!(result, "JUNE");
    }

    #[test]
    fn test_swap_case_flag() {
        let date = make_test_date(1999, 6, 1, 0);
        let config = get_config();
        // %#B: swap case on "June" (mixed case) → uppercase
        let result = format_with_modifiers(&date, "%#B", &config).unwrap();
        assert_eq!(result, "JUNE");
    }

    #[test]
    fn test_width_smaller_than_result() {
        let date = make_test_date(1999, 6, 1, 0);
        let config = get_config();
        // %1d: width 1 < "01".len() → strip zero padding → "1"
        let result = format_with_modifiers(&date, "%1d", &config).unwrap();
        assert_eq!(result, "1");
    }

    #[test]
    fn test_edge_cases_and_special_formats() {
        let date = make_test_date(1999, 6, 1, 0);
        let config = get_config();

        // Test width zero (no effect)
        let result = format_with_modifiers(&date, "%Y", &config).unwrap();
        assert_eq!(result, "1999");

        // Test no modifiers (standard format)
        let result = format_with_modifiers(&date, "%Y-%m-%d", &config).unwrap();
        assert_eq!(result, "1999-06-01");

        // Test %% escape sequence
        let result = format_with_modifiers(&date, "%%Y=%Y", &config).unwrap();
        assert_eq!(result, "%Y=1999");

        // Test multiple modifiers in one format string
        // %-5d: no-pad suppresses all padding → "1" (width ignored)
        let result = format_with_modifiers(&date, "%10Y-%_5m-%-5d", &config).unwrap();
        assert_eq!(result, "0000001999-    6-1");
    }

    #[test]
    fn test_modifier_detection() {
        let date = make_test_date(1999, 6, 1, 0);
        let config = get_config();

        // Should detect modifiers
        let result = format_with_modifiers_if_present(&date, "%10Y", &config);
        assert!(result.is_some());

        // Should not detect modifiers
        let result = format_with_modifiers_if_present(&date, "%Y-%m-%d", &config);
        assert!(result.is_none());

        // Should detect flag without width
        let result = format_with_modifiers_if_present(&date, "%^B", &config);
        assert!(result.is_some());
    }

    #[test]
    fn test_negative_values_with_space_padding() {
        // Test case from GNU test: neg-secs2
        // Format: %_5s with value -22 should produce "  -22" (space-padded)
        use jiff::Timestamp;

        let ts = Timestamp::from_second(-22).unwrap();
        let date = ts.to_zoned(TimeZone::UTC);
        let config = get_config();

        let result = format_with_modifiers(&date, "%_5s", &config).unwrap();
        assert_eq!(
            result, "  -22",
            "Space padding should pad before the sign for negative numbers"
        );
    }

    // Unit tests for apply_modifiers function
    #[test]
    fn test_apply_modifiers_basic() {
        // No modifiers (numeric specifier)
        assert_eq!(apply_modifiers("1999", "", 0, "Y").unwrap(), "1999");
        // Zero padding
        assert_eq!(apply_modifiers("1999", "0", 10, "Y").unwrap(), "0000001999");
        // Space padding (strips leading zeros)
        assert_eq!(apply_modifiers("06", "_", 5, "m").unwrap(), "    6");
        // No-pad (strips leading zeros, width ignored)
        assert_eq!(apply_modifiers("01", "-", 5, "d").unwrap(), "1");
        // Uppercase
        assert_eq!(apply_modifiers("june", "^", 0, "B").unwrap(), "JUNE");
        // Swap case: all uppercase → lowercase
        assert_eq!(apply_modifiers("UTC", "#", 0, "Z").unwrap(), "utc");
        // Swap case: mixed case → uppercase
        assert_eq!(apply_modifiers("June", "#", 0, "B").unwrap(), "JUNE");
    }

    #[test]
    fn test_apply_modifiers_signs() {
        // Force sign
        assert_eq!(apply_modifiers("1970", "+", 6, "Y").unwrap(), "+01970");
        // Negative with zero padding: sign first, then zeros
        assert_eq!(apply_modifiers("-22", "0", 5, "s").unwrap(), "-0022");
        // Negative with space padding: spaces first, then sign
        assert_eq!(apply_modifiers("-22", "_", 5, "s").unwrap(), "  -22");
        // Force sign (_+): + is last, overrides _ → zero pad with sign
        assert_eq!(apply_modifiers("5", "_+", 5, "s").unwrap(), "+0005");
        // No-pad + uppercase: no padding applied
        assert_eq!(apply_modifiers("june", "-^", 10, "B").unwrap(), "JUNE");
    }

    #[test]
    fn test_case_flag_precedence() {
        // Test that ^ (uppercase) overrides # (swap case)
        assert_eq!(apply_modifiers("June", "^#", 0, "B").unwrap(), "JUNE");
        assert_eq!(apply_modifiers("June", "#^", 0, "B").unwrap(), "JUNE");
        // Test # alone (swap case)
        assert_eq!(apply_modifiers("June", "#", 0, "B").unwrap(), "JUNE");
        assert_eq!(apply_modifiers("JUNE", "#", 0, "B").unwrap(), "june");
    }

    #[test]
    fn test_apply_modifiers_text_specifiers() {
        // Text specifiers default to space padding
        assert_eq!(apply_modifiers("June", "", 10, "B").unwrap(), "      June");
        assert_eq!(apply_modifiers("Mon", "", 10, "a").unwrap(), "       Mon");
        // Numeric specifiers default to zero padding
        assert_eq!(apply_modifiers("6", "", 10, "m").unwrap(), "0000000006");
    }

    #[test]
    fn test_apply_modifiers_width_smaller_than_result() {
        // Width smaller than result strips default padding
        assert_eq!(apply_modifiers("01", "", 1, "d").unwrap(), "1");
        assert_eq!(apply_modifiers("06", "", 1, "m").unwrap(), "6");
    }

    #[test]
    fn test_apply_modifiers_parametrized() {
        let test_cases = vec![
            ("1", "0", 3, "Y", "001"),
            ("1", "_", 3, "d", "  1"),
            ("1", "-", 3, "d", "1"),       // no-pad: width ignored
            ("abc", "^", 5, "B", "  ABC"), // text specifier: space pad
            ("5", "+", 4, "s", "+005"),
            ("5", "_+", 4, "s", "+005"), // + is last: zero pad with sign
            ("-3", "0", 5, "s", "-0003"),
            ("05", "_", 3, "d", "  5"),
            ("09", "-", 4, "d", "9"),         // no-pad: width ignored
            ("1970", "_+", 6, "Y", "+01970"), // + is last: zero pad with sign
        ];

        for (value, flags, width, spec, expected) in test_cases {
            assert_eq!(
                apply_modifiers(value, flags, width, spec).unwrap(),
                expected,
                "value='{value}', flags='{flags}', width={width}, spec='{spec}'",
            );
        }
    }

    #[test]
    fn test_apply_modifiers_width_too_large() {
        let err = apply_modifiers("x", "", usize::MAX, "c").unwrap_err();
        assert!(matches!(
            err,
            FormatError::Custom(message) if message == ERR_FIELD_WIDTH_TOO_LARGE
        ));
    }
}
