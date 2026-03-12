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
    /// Custom error message (reserved for future use)
    #[allow(dead_code)]
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
            let explicit_width = !width_str.is_empty();
            let modified = apply_modifiers(&formatted, flags, width, spec, explicit_width);
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

/// Returns true if the specifier defaults to space padding.
/// This includes text specifiers and numeric specifiers like %e and %k
/// that use blank-padding by default in GNU date.
fn is_space_padded_specifier(specifier: &str) -> bool {
    matches!(
        specifier.chars().last(),
        Some('A' | 'a' | 'B' | 'b' | 'h' | 'Z' | 'p' | 'P' | 'e' | 'k' | 'l')
    )
}

/// Returns the default width for a specifier.
/// This is used when a flag like `_` is used without an explicit width.
fn get_default_width(specifier: &str) -> usize {
    match specifier.chars().last() {
        // Day of month: 2 digits (01-31)
        Some('d') | Some('e') => 2,
        // Month: 2 digits (01-12)
        Some('m') => 2,
        // Hour: 2 digits (00-23)
        Some('H') | Some('k') => 2,
        // Hour (12-hour): 2 digits (01-12)
        Some('I') | Some('l') => 2,
        // Minute: 2 digits (00-59)
        Some('M') => 2,
        // Second: 2 digits (00-60)
        Some('S') => 2,
        // Year (2-digit): 2 digits
        Some('y') => 2,
        // Day of year: 3 digits (001-366)
        Some('j') => 3,
        // Week number: 2 digits (00-53)
        Some('U') | Some('W') | Some('V') => 2,
        // Day of week: 1 digit (0-6 or 1-7)
        Some('w') | Some('u') => 1,
        // Century: 2 digits (00-99)
        Some('C') => 2,
        // Full year: 4 digits
        Some('Y') | Some('G') => 4,
        // ISO week year (2-digit): 2 digits
        Some('g') => 2,
        // Epoch seconds: typically 10 digits (but variable)
        Some('s') => 0,
        // Nanoseconds: 9 digits
        Some('N') => 9,
        // Quarter: 1 digit
        Some('q') => 1,
        // Timezone offset: varies
        Some('z') => 0,
        // Text specifiers have no default width
        _ => 0,
    }
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
///
/// The `explicit_width` parameter indicates whether a width was explicitly
/// specified in the format string (true) or if width is 0 (false).
fn apply_modifiers(
    value: &str,
    flags: &str,
    width: usize,
    specifier: &str,
    explicit_width: bool,
) -> String {
    let mut result = value.to_string();

    // Determine default pad character based on specifier type
    // Determine default pad character based on specifier type.
    // Text specifiers (month names, etc.) and numeric specifiers like %e, %k, %l
    // default to space padding; other numeric specifiers default to zero padding.
    let default_pad = if is_space_padded_specifier(specifier) {
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
    let mut underscore_flag = false;

    for flag in flags.chars() {
        match flag {
            '-' => {
                no_pad = true;
            }
            '_' => {
                no_pad = false;
                pad_char = ' ';
                underscore_flag = true;
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
        return strip_default_padding(&result);
    }

    // Handle padding flag without explicit width: use default width
    // This applies when _ or 0 flag overrides the default padding character
    // and no explicit width is specified (e.g., %_m, %0e)
    let effective_width = if !explicit_width && (underscore_flag || pad_char != default_pad) {
        get_default_width(specifier)
    } else {
        width
    };

    // Handle width smaller than result: strip default padding to fit
    if effective_width > 0 && effective_width < result.len() {
        return strip_default_padding(&result);
    }

    // Strip default padding when switching pad characters on numeric fields
    if !is_text_specifier(specifier) && result.len() >= 2 {
        if pad_char == ' ' && result.starts_with('0') {
            // Switching to space padding: strip leading zeros
            result = strip_default_padding(&result);
        } else if pad_char == '0' && result.starts_with(' ') {
            // Switching to zero padding: strip leading spaces
            result = strip_default_padding(&result);
        }
    }

    // Apply force sign for numeric values
    // GNU behavior: + only adds sign if:
    // 1. An explicit width is provided, OR
    // 2. The value exceeds the default width for that specifier (e.g., year > 4 digits)
    if force_sign && !result.starts_with('+') && !result.starts_with('-') {
        if result.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            let default_w = get_default_width(specifier);
            // Add sign only if explicit width provided OR result exceeds default width
            if explicit_width || (default_w > 0 && result.len() > default_w) {
                result.insert(0, '+');
            }
        }
    }

    // Apply width padding
    if effective_width > result.len() {
        let padding = effective_width - result.len();
        let has_sign = result.starts_with('+') || result.starts_with('-');

        if pad_char == '0' && has_sign {
            // Zero padding: sign first, then zeros (e.g., "-0022")
            let sign = result.chars().next().unwrap();
            let rest = &result[1..];
            result = format!("{sign}{}{rest}", "0".repeat(padding));
        } else {
            // Default: pad on the left (e.g., "  -22" or "  1999")
            result = format!("{}{result}", pad_char.to_string().repeat(padding));
        }
    }

    result
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
        assert_eq!(apply_modifiers("1999", "", 0, "Y", false), "1999");
        // Zero padding
        assert_eq!(apply_modifiers("1999", "0", 10, "Y", true), "0000001999");
        // Space padding (strips leading zeros)
        assert_eq!(apply_modifiers("06", "_", 5, "m", true), "    6");
        // No-pad (strips leading zeros, width ignored)
        assert_eq!(apply_modifiers("01", "-", 5, "d", true), "1");
        // Uppercase
        assert_eq!(apply_modifiers("june", "^", 0, "B", false), "JUNE");
        // Swap case: all uppercase → lowercase
        assert_eq!(apply_modifiers("UTC", "#", 0, "Z", false), "utc");
        // Swap case: mixed case → uppercase
        assert_eq!(apply_modifiers("June", "#", 0, "B", false), "JUNE");
    }

    #[test]
    fn test_apply_modifiers_signs() {
        // Force sign with explicit width
        assert_eq!(apply_modifiers("1970", "+", 6, "Y", true), "+01970");
        // Force sign without explicit width: should NOT add sign for 4-digit year
        assert_eq!(apply_modifiers("1999", "+", 0, "Y", false), "1999");
        // Force sign without explicit width: SHOULD add sign for year > 4 digits
        assert_eq!(apply_modifiers("12345", "+", 0, "Y", false), "+12345");
        // Negative with zero padding: sign first, then zeros
        assert_eq!(apply_modifiers("-22", "0", 5, "s", true), "-0022");
        // Negative with space padding: spaces first, then sign
        assert_eq!(apply_modifiers("-22", "_", 5, "s", true), "  -22");
        // Force sign (_+): + is last, overrides _ → zero pad with sign
        assert_eq!(apply_modifiers("5", "_+", 5, "s", true), "+0005");
        // No-pad + uppercase: no padding applied
        assert_eq!(apply_modifiers("june", "-^", 10, "B", true), "JUNE");
    }

    #[test]
    fn test_case_flag_precedence() {
        // Test that ^ (uppercase) overrides # (swap case)
        assert_eq!(apply_modifiers("June", "^#", 0, "B", false), "JUNE");
        assert_eq!(apply_modifiers("June", "#^", 0, "B", false), "JUNE");
        // Test # alone (swap case)
        assert_eq!(apply_modifiers("June", "#", 0, "B", false), "JUNE");
        assert_eq!(apply_modifiers("JUNE", "#", 0, "B", false), "june");
    }

    #[test]
    fn test_apply_modifiers_text_specifiers() {
        // Text specifiers default to space padding
        assert_eq!(apply_modifiers("June", "", 10, "B", true), "      June");
        assert_eq!(apply_modifiers("Mon", "", 10, "a", true), "       Mon");
        // Numeric specifiers default to zero padding
        assert_eq!(apply_modifiers("6", "", 10, "m", true), "0000000006");
    }

    #[test]
    fn test_apply_modifiers_width_smaller_than_result() {
        // Width smaller than result strips default padding
        assert_eq!(apply_modifiers("01", "", 1, "d", true), "1");
        assert_eq!(apply_modifiers("06", "", 1, "m", true), "6");
    }

    #[test]
    fn test_apply_modifiers_parametrized() {
        let test_cases = vec![
            ("1", "0", 3, "Y", true, "001"),
            ("1", "_", 3, "d", true, "  1"),
            ("1", "-", 3, "d", true, "1"), // no-pad: width ignored
            ("abc", "^", 5, "B", true, "  ABC"), // text specifier: space pad
            ("5", "+", 4, "s", true, "+005"),
            ("5", "_+", 4, "s", true, "+005"), // + is last: zero pad with sign
            ("-3", "0", 5, "s", true, "-0003"),
            ("05", "_", 3, "d", true, "  5"),
            ("09", "-", 4, "d", true, "9"), // no-pad: width ignored
            ("1970", "_+", 6, "Y", true, "+01970"), // + is last: zero pad with sign
        ];

        for (value, flags, width, spec, explicit_width, expected) in test_cases {
            assert_eq!(
                apply_modifiers(value, flags, width, spec, explicit_width),
                expected,
                "value='{value}', flags='{flags}', width={width}, spec='{spec}', explicit_width={explicit_width}",
            );
        }
    }

    #[test]
    fn test_underscore_flag_without_width() {
        // %_m should pad month to default width 2 with spaces
        assert_eq!(apply_modifiers("6", "_", 0, "m", false), " 6");
        // %_d should pad day to default width 2 with spaces
        assert_eq!(apply_modifiers("1", "_", 0, "d", false), " 1");
        // %_H should pad hour to default width 2 with spaces
        assert_eq!(apply_modifiers("5", "_", 0, "H", false), " 5");
        // %_Y should pad year to default width 4 with spaces
        assert_eq!(apply_modifiers("1999", "_", 0, "Y", false), "1999"); // already at default width
    }

    #[test]
    fn test_plus_flag_without_width() {
        // %+Y without width should NOT add sign for 4-digit year
        assert_eq!(apply_modifiers("1999", "+", 0, "Y", false), "1999");
        // %+Y without width SHOULD add sign for year > 4 digits
        assert_eq!(apply_modifiers("12345", "+", 0, "Y", false), "+12345");
        // %+Y with explicit width should add sign
        assert_eq!(apply_modifiers("1999", "+", 6, "Y", true), "+01999");
    }

    #[test]
    fn test_zero_flag_on_space_padded_specifiers() {
        // GNU date: %0e should override space-padding with zero-padding
        // Verified: `date -d "2024-06-05" "+%0e"` → "05"
        let date = make_test_date(1999, 6, 5, 5);
        let config = get_config();

        // %0e: day-of-month (normally space-padded) with 0 flag → zero-padded
        let result = format_with_modifiers(&date, "%0e", &config).unwrap();
        assert_eq!(result, "05", "GNU: %0e should produce '05', not ' 5'");

        // %0k: hour (normally space-padded) with 0 flag → zero-padded
        let result = format_with_modifiers(&date, "%0k", &config).unwrap();
        assert_eq!(result, "05", "GNU: %0k should produce '05', not ' 5'");
    }

    #[test]
    fn test_underscore_century_default_width() {
        // GNU date: %C default width is 2, not 4
        // Verified: `date -d "2024-06-15" "+%_C"` → "20" (no extra padding)
        let date = make_test_date(1999, 6, 1, 0);
        let config = get_config();

        // %_C: century with underscore flag, no explicit width
        // Default width for %C should be 2 (century is 00-99)
        let result = format_with_modifiers(&date, "%_C", &config).unwrap();
        assert_eq!(
            result, "19",
            "GNU: %_C should produce '19', not '  19' (default width is 2, not 4)"
        );
    }
}
