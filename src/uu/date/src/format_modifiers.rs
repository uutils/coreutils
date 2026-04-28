// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore strtime Yhello

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
use std::fmt;
use uucore::translate;

/// Error type for format modifier operations
#[derive(Debug)]
pub enum FormatError {
    /// Error from the underlying jiff library
    JiffError(jiff::Error),
    /// Field width calculation overflowed or required allocation failed
    FieldWidthTooLarge { width: usize, specifier: String },
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::JiffError(e) => write!(f, "{e}"),
            Self::FieldWidthTooLarge { width, specifier } => write!(
                f,
                "{}",
                translate!(
                    "date-error-format-modifier-width-too-large",
                    "width" => width,
                    "specifier" => specifier
                )
            ),
        }
    }
}

impl From<jiff::Error> for FormatError {
    fn from(e: jiff::Error) -> Self {
        Self::JiffError(e)
    }
}

/// A parsed `%`-format specifier: `%[flags][width][:colons]<letter>`.
struct ParsedSpec<'a> {
    /// Flag characters from `[_0^#+-]`.
    flags: &'a str,
    /// Explicit width, if present. `None` means no width was specified.
    /// A value that overflows `usize` is represented as `Some(usize::MAX)` so
    /// the downstream allocation check surfaces it as `FieldWidthTooLarge`.
    width: Option<usize>,
    /// The specifier itself, including any leading colons (e.g. `Y`, `:z`, `::z`).
    spec: &'a str,
    /// Total byte length of the parsed sequence including the leading `%`.
    len: usize,
}

/// Try to parse a format spec at the start of `s`.
///
/// Implements the grammar `%[_0^#+-]*[0-9]*:*[a-zA-Z]` anchored at the
/// beginning of `s`. Returns `None` if `s` does not start with `%` or no
/// valid specifier follows.
fn parse_format_spec(s: &str) -> Option<ParsedSpec<'_>> {
    let bytes = s.as_bytes();
    if bytes.first() != Some(&b'%') {
        return None;
    }

    let mut pos = 1;

    // Flags: any of [_0^#+-], zero or more.
    let flags_start = pos;
    while pos < bytes.len()
        && matches!(
            bytes[pos],
            b'_' | b'0' | b'^' | b'#' | b'+' | b'-' | b'O' | b'E'
        )
    {
        pos += 1;
    }
    let flags = &s[flags_start..pos];

    // Width: zero or more ASCII digits.
    let width_start = pos;
    while pos < bytes.len() && bytes[pos].is_ascii_digit() {
        pos += 1;
    }
    let width = if pos > width_start {
        Some(s[width_start..pos].parse::<usize>().unwrap_or(usize::MAX))
    } else {
        None
    };

    // Specifier: zero or more `:` followed by a single ASCII letter.
    let spec_start = pos;
    while pos < bytes.len() && bytes[pos] == b':' {
        pos += 1;
    }
    if pos >= bytes.len() || !bytes[pos].is_ascii_alphabetic() {
        return None;
    }
    pos += 1;
    let spec = &s[spec_start..pos];

    Some(ParsedSpec {
        flags,
        width,
        spec,
        len: pos,
    })
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
    if !has_gnu_modifiers(format_string) {
        return None;
    }

    // If we have modifiers, format the string
    Some(format_with_modifiers(date, format_string, config))
}

/// Quick check: does the format string contain any GNU modifier
/// (a flag or width) on a `%`-spec, ignoring `%%` literals?
///
/// Note that colon-prefixed specifiers without flags or width (e.g. `%:z`,
/// `%::z`) are deliberately *not* considered modifiers: jiff's strftime can
/// format them directly, so the caller can take the standard fast path.
fn has_gnu_modifiers(format_string: &str) -> bool {
    let bytes = format_string.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            // Skip %% literal
            if bytes.get(i + 1) == Some(&b'%') {
                i += 2;
                continue;
            }
            if let Some(parsed) = parse_format_spec(&format_string[i..]) {
                if !parsed.flags.is_empty() || parsed.width.is_some() {
                    return true;
                }
                i += parsed.len;
                continue;
            }
        }
        i += 1;
    }
    false
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
    let mut result = String::new();
    let broken_down = BrokenDownTime::from(date);

    // Reused across iterations to avoid allocating a fresh `String` per spec.
    // Holds the leading `%` plus the specifier itself (e.g. `%Y`, `%::z`),
    // which is at most a handful of bytes.
    let mut base_format = String::with_capacity(8);

    let bytes = format_string.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            // Handle %% literal: emit a single '%' and continue.
            if bytes.get(i + 1) == Some(&b'%') {
                result.push('%');
                i += 2;
                continue;
            }

            if let Some(parsed) = parse_format_spec(&format_string[i..]) {
                // Format the base specifier first, reusing `base_format`.
                base_format.clear();
                base_format.push('%');
                base_format.push_str(parsed.spec);
                let mut formatted = String::new();
                // If modifier is 'E' or 'O',
                // check if specifier is allowed to work with the modifier.
                if parsed.flags == "E" {
                    // Modifier applies to the ‘%c’, ‘%C’, ‘%x’, ‘%X’,
                    // ‘%y’ and ‘%Y’ conversion specifiers.
                    if matches!(parsed.spec, "c" | "C" | "x" | "X" | "y" | "Y") {
                        formatted.push_str(
                            broken_down
                                .to_string_with_config(config, &base_format)?
                                .as_str(),
                        );
                    } else {
                        // Use unformatted string to display
                        // if specifier does not work with modifier 'E'.
                        formatted.push('%');
                        formatted.push_str(parsed.flags);
                        formatted.push_str(parsed.spec);
                    }
                } else if parsed.flags == "O" {
                    // Modifier 'O' applies only to numeric conversion specifiers.
                    if matches!(parsed.spec, "a" | "A" | "c" | "D" | "F" | "x" | "X") {
                        // Use unformatted string to display
                        // if specifier does not work with modifier 'O'.
                        formatted.push('%');
                        formatted.push_str(parsed.flags);
                        formatted.push_str(parsed.spec);
                    } else {
                        // All other specifiers work with modifier 'O'.
                        formatted.push_str(
                            broken_down
                                .to_string_with_config(config, &base_format)?
                                .as_str(),
                        );
                    }
                } else {
                    // If modifier is not 'E' either 'O', format the string with config.
                    formatted.push_str(
                        broken_down
                            .to_string_with_config(config, &base_format)?
                            .as_str(),
                    );
                }

                if !parsed.flags.is_empty() || parsed.width.is_some() {
                    let modified = apply_modifiers(&formatted, &parsed)?;
                    result.push_str(&modified);
                } else {
                    result.push_str(&formatted);
                }

                i += parsed.len;
                continue;
            }
        }

        // Pass-through: copy a single UTF-8 code point unchanged.
        let ch_len = format_string[i..].chars().next().map_or(1, char::len_utf8);
        result.push_str(&format_string[i..i + ch_len]);
        i += ch_len;
    }

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
/// The specifier inside `parsed` (e.g., "d", "B", "Y") determines the default
/// padding character (space for text, zero for numeric).
/// Flags are processed in order so that when conflicting flags appear,
/// the last one takes precedence (e.g., `_+` means `+` wins for padding).
fn apply_modifiers(value: &str, parsed: &ParsedSpec<'_>) -> Result<String, FormatError> {
    let flags = parsed.flags;
    let width = parsed.width;
    let specifier = parsed.spec;
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
            '#' if !uppercase => {
                // Only apply # if ^ hasn't been set
                swap_case = true;
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
        } else if !result
            .chars()
            .all(|c| !c.is_alphabetic() || c.is_lowercase())
        {
            result = result.to_uppercase();
        }
    }

    // If no_pad flag is active, suppress all padding and return
    if no_pad {
        return Ok(strip_default_padding(&result));
    }

    // Handle padding flag without explicit width: use default width.
    // This applies when _ or 0 flag overrides the default padding character
    // and no explicit width is specified (e.g., %_m, %0e).
    let effective_width = match width {
        Some(w) => w,
        None if underscore_flag || pad_char != default_pad => get_default_width(specifier),
        None => 0,
    };

    // When the requested width is narrower than the default formatted width, GNU first removes default padding and then reapplies the requested width.
    if effective_width > 0 && effective_width < result.len() {
        result = strip_default_padding(&result);
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
            if width.is_some() || (default_w > 0 && result.len() > default_w) {
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
            let mut padded = try_alloc_padded(result.len(), padding, effective_width, specifier)?;
            padded.push(sign);
            padded.extend(std::iter::repeat_n('0', padding));
            padded.push_str(rest);
            result = padded;
        } else {
            // Default: pad on the left (e.g., "  -22" or "  1999")
            let mut padded = try_alloc_padded(result.len(), padding, effective_width, specifier)?;
            padded.extend(std::iter::repeat_n(pad_char, padding));
            padded.push_str(&result);
            result = padded;
        }
    } else if specifier.ends_with('N') {
        if effective_width <= get_default_width(specifier) && effective_width != 0 {
            result.truncate(effective_width);
        }
    }

    Ok(result)
}

/// Allocate a `String` with enough capacity for `current_len + padding`,
/// returning `FieldWidthTooLarge` on arithmetic overflow or allocation failure.
fn try_alloc_padded(
    current_len: usize,
    padding: usize,
    width: usize,
    specifier: &str,
) -> Result<String, FormatError> {
    let target_len =
        current_len
            .checked_add(padding)
            .ok_or_else(|| FormatError::FieldWidthTooLarge {
                width,
                specifier: specifier.to_string(),
            })?;
    let mut s = String::new();
    s.try_reserve(target_len)
        .map_err(|_| FormatError::FieldWidthTooLarge {
            width,
            specifier: specifier.to_string(),
        })?;
    Ok(s)
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

    /// Build a `ParsedSpec` for unit-testing `apply_modifiers` without a real
    /// format string.  `len` is set to 0 because these tests never use it.
    fn spec<'a>(flags: &'a str, width: Option<usize>, spec: &'a str) -> ParsedSpec<'a> {
        ParsedSpec {
            flags,
            width,
            spec,
            len: 0,
        }
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
        assert_eq!(
            apply_modifiers("1999", &spec("", None, "Y")).unwrap(),
            "1999"
        );
        // Zero padding
        assert_eq!(
            apply_modifiers("1999", &spec("0", Some(10), "Y")).unwrap(),
            "0000001999"
        );
        // Space padding (strips leading zeros)
        assert_eq!(
            apply_modifiers("06", &spec("_", Some(5), "m")).unwrap(),
            "    6"
        );
        // No-pad (strips leading zeros, width ignored)
        assert_eq!(
            apply_modifiers("01", &spec("-", Some(5), "d")).unwrap(),
            "1"
        );
        // Uppercase
        assert_eq!(
            apply_modifiers("june", &spec("^", None, "B")).unwrap(),
            "JUNE"
        );
        // Swap case: all uppercase → lowercase
        assert_eq!(
            apply_modifiers("UTC", &spec("#", None, "Z")).unwrap(),
            "utc"
        );
        // Swap case: mixed case → uppercase
        assert_eq!(
            apply_modifiers("June", &spec("#", None, "B")).unwrap(),
            "JUNE"
        );
    }

    #[test]
    fn test_apply_modifiers_signs() {
        // Force sign with explicit width
        assert_eq!(
            apply_modifiers("1970", &spec("+", Some(6), "Y")).unwrap(),
            "+01970"
        );
        // Force sign without explicit width: should NOT add sign for 4-digit year
        assert_eq!(
            apply_modifiers("1999", &spec("+", None, "Y")).unwrap(),
            "1999"
        );
        // Force sign without explicit width: SHOULD add sign for year > 4 digits
        assert_eq!(
            apply_modifiers("12345", &spec("+", None, "Y")).unwrap(),
            "+12345"
        );
        // Negative with zero padding: sign first, then zeros
        assert_eq!(
            apply_modifiers("-22", &spec("0", Some(5), "s")).unwrap(),
            "-0022"
        );
        // Negative with space padding: spaces first, then sign
        assert_eq!(
            apply_modifiers("-22", &spec("_", Some(5), "s")).unwrap(),
            "  -22"
        );
        // Force sign (_+): + is last, overrides _ → zero pad with sign
        assert_eq!(
            apply_modifiers("5", &spec("_+", Some(5), "s")).unwrap(),
            "+0005"
        );
        // No-pad + uppercase: no padding applied
        assert_eq!(
            apply_modifiers("june", &spec("-^", Some(10), "B")).unwrap(),
            "JUNE"
        );
    }

    #[test]
    fn test_case_flag_precedence() {
        // Test that ^ (uppercase) overrides # (swap case)
        assert_eq!(
            apply_modifiers("June", &spec("^#", None, "B")).unwrap(),
            "JUNE"
        );
        assert_eq!(
            apply_modifiers("June", &spec("#^", None, "B")).unwrap(),
            "JUNE"
        );
        // Test # alone (swap case)
        assert_eq!(
            apply_modifiers("June", &spec("#", None, "B")).unwrap(),
            "JUNE"
        );
        assert_eq!(
            apply_modifiers("JUNE", &spec("#", None, "B")).unwrap(),
            "june"
        );
    }

    #[test]
    fn test_apply_modifiers_text_specifiers() {
        // Text specifiers default to space padding
        assert_eq!(
            apply_modifiers("June", &spec("", Some(10), "B")).unwrap(),
            "      June"
        );
        assert_eq!(
            apply_modifiers("Mon", &spec("", Some(10), "a")).unwrap(),
            "       Mon"
        );
        // Numeric specifiers default to zero padding
        assert_eq!(
            apply_modifiers("6", &spec("", Some(10), "m")).unwrap(),
            "0000000006"
        );
    }

    #[test]
    fn test_apply_modifiers_width_smaller_than_result() {
        // Width smaller than result strips default padding
        assert_eq!(apply_modifiers("01", &spec("", Some(1), "d")).unwrap(), "1");
        assert_eq!(apply_modifiers("06", &spec("", Some(1), "m")).unwrap(), "6");
    }

    #[test]
    fn test_apply_modifiers_parametrized() {
        let test_cases = vec![
            ("1", "0", Some(3), "Y", "001"),
            ("1", "_", Some(3), "d", "  1"),
            ("1", "-", Some(3), "d", "1"), // no-pad: width ignored
            ("abc", "^", Some(5), "B", "  ABC"), // text specifier: space pad
            ("5", "+", Some(4), "s", "+005"),
            ("5", "_+", Some(4), "s", "+005"), // + is last: zero pad with sign
            ("-3", "0", Some(5), "s", "-0003"),
            ("05", "_", Some(3), "d", "  5"),
            ("09", "-", Some(4), "d", "9"), // no-pad: width ignored
            ("1970", "_+", Some(6), "Y", "+01970"), // + is last: zero pad with sign
        ];

        for (value, flags, width, s, expected) in test_cases {
            let p = spec(flags, width, s);
            assert_eq!(
                apply_modifiers(value, &p).unwrap(),
                expected,
                "value='{value}', flags='{flags}', width={width:?}, spec='{s}'",
            );
        }
    }

    #[test]
    fn test_apply_modifiers_width_too_large() {
        let err = apply_modifiers("x", &spec("", Some(usize::MAX), "c")).unwrap_err();
        assert!(matches!(
            err,
            FormatError::FieldWidthTooLarge { width, specifier }
            if width == usize::MAX && specifier == "c"
        ));
    }

    #[test]
    fn test_format_with_modifiers_width_overflows_usize() {
        // A width literal that overflows `usize` must surface as
        // `FieldWidthTooLarge` (via the downstream allocation check),
        // not silently fall back to width 0.
        let date = make_test_date(1999, 6, 1, 0);
        let config = get_config();
        let huge = "9".repeat(40);
        let format = format!("%{huge}Y");
        let err = format_with_modifiers(&date, &format, &config).unwrap_err();
        assert!(matches!(
            err,
            FormatError::FieldWidthTooLarge { width, specifier }
            if width == usize::MAX && specifier == "Y"
        ));
    }

    #[test]
    fn test_underscore_flag_without_width() {
        // %_m should pad month to default width 2 with spaces
        assert_eq!(apply_modifiers("6", &spec("_", None, "m")).unwrap(), " 6");
        // %_d should pad day to default width 2 with spaces
        assert_eq!(apply_modifiers("1", &spec("_", None, "d")).unwrap(), " 1");
        // %_H should pad hour to default width 2 with spaces
        assert_eq!(apply_modifiers("5", &spec("_", None, "H")).unwrap(), " 5");
        // %_Y should pad year to default width 4 with spaces
        assert_eq!(
            apply_modifiers("1999", &spec("_", None, "Y")).unwrap(),
            "1999"
        );
        // already at default width
    }

    #[test]
    fn test_plus_flag_without_width() {
        // %+Y without width should NOT add sign for 4-digit year
        assert_eq!(
            apply_modifiers("1999", &spec("+", None, "Y")).unwrap(),
            "1999"
        );
        // %+Y without width SHOULD add sign for year > 4 digits
        assert_eq!(
            apply_modifiers("12345", &spec("+", None, "Y")).unwrap(),
            "+12345"
        );
        // %+Y with explicit width should add sign
        assert_eq!(
            apply_modifiers("1999", &spec("+", Some(6), "Y")).unwrap(),
            "+01999"
        );
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

    #[test]
    fn test_parse_format_spec() {
        // (input, expected: Some((flags, width, spec, len)) or None)
        type ParsedTuple = (&'static str, Option<usize>, &'static str, usize);
        let cases: &[(&str, Option<ParsedTuple>)] = &[
            // ---- plain single-letter specifiers ----
            ("%Y", Some(("", None, "Y", 2))),
            ("%a", Some(("", None, "a", 2))),
            ("%B", Some(("", None, "B", 2))),
            // ---- single flag, no width ----
            ("%-d", Some(("-", None, "d", 3))),
            ("%_m", Some(("_", None, "m", 3))),
            ("%0e", Some(("0", None, "e", 3))),
            ("%^B", Some(("^", None, "B", 3))),
            ("%#Z", Some(("#", None, "Z", 3))),
            ("%+Y", Some(("+", None, "Y", 3))),
            // ---- combined flags ----
            ("%_+Y", Some(("_+", None, "Y", 4))),
            ("%-^B", Some(("-^", None, "B", 4))),
            // ---- width only ----
            ("%10Y", Some(("", Some(10), "Y", 4))),
            ("%4C", Some(("", Some(4), "C", 3))),
            // `0` is a flag, then `5` is the width.
            ("%05d", Some(("0", Some(5), "d", 4))),
            // ---- flags + width ----
            ("%_10m", Some(("_", Some(10), "m", 5))),
            ("%+6Y", Some(("+", Some(6), "Y", 4))),
            ("%-5d", Some(("-", Some(5), "d", 4))),
            ("%+4C", Some(("+", Some(4), "C", 4))),
            // ---- colon-prefixed specifiers (numeric timezones) ----
            ("%:z", Some(("", None, ":z", 3))),
            ("%::z", Some(("", None, "::z", 4))),
            ("%:::z", Some(("", None, ":::z", 5))),
            ("%-3:z", Some(("-", Some(3), ":z", 5))),
            // ---- only the spec is consumed; trailing text is ignored ----
            ("%Y-%m-%d", Some(("", None, "Y", 2))),
            ("%10Yhello", Some(("", Some(10), "Y", 4))),
            // ---- invalid: should return None ----
            ("Y", None),
            ("", None),
            ("%", None),
            ("%-", None),
            ("%10", None),
            ("%_+", None),
            ("%:", None),
            ("%::", None),
            ("%%", None), // %% is not a spec — caller handles it.
            ("%é", None), // non-ASCII letter
        ];

        for (input, expected) in cases {
            let actual = parse_format_spec(input).map(|p| (p.flags, p.width, p.spec, p.len));
            assert_eq!(actual, *expected, "input = {input:?}");
        }
    }

    #[test]
    fn test_has_gnu_modifiers() {
        // (input, expected)
        let cases: &[(&str, bool)] = &[
            // ---- modifier present (flag and/or width) ----
            ("%10Y", true),
            ("%^B", true),
            ("%-d", true),
            ("%_m", true),
            ("%+Y", true),
            ("today is %-d of %B", true),
            ("%5:z", true),
            // %% mixed with a real modifier is still detected.
            ("%%%-d", true),
            ("%%%10Y", true),
            // ---- no modifier: plain specs only ----
            ("%Y-%m-%d", false),
            ("%H:%M:%S", false),
            ("%a %b %e %T %Z %Y", false),
            ("", false),
            ("no percent here", false),
            // ---- %% literals must never count as modifiers ----
            ("%%", false),
            ("100%% done", false),
            ("%%Y", false),
            ("%%10", false),
            // ---- colon specs without flags/width are not modifiers ----
            ("%:z", false),
            ("%::z", false),
        ];

        for (input, expected) in cases {
            assert_eq!(has_gnu_modifiers(input), *expected, "input = {input:?}");
        }
    }
}
