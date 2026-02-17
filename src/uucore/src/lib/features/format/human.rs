// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore gnulibs sfmt

//! `human`-size formatting
//!
//! Format sizes like gnulibs human_readable() would

use unit_prefix::NumberPrefix;

#[derive(Copy, Clone, PartialEq)]
pub enum SizeFormat {
    Bytes,
    Binary,  // Powers of 1024, --human-readable, -h
    Decimal, // Powers of 1000, --si
}

/// There are a few peculiarities to how GNU formats the sizes:
/// 1. One decimal place is given if and only if the size is smaller than 10
/// 2. It rounds sizes up.
/// 3. The human-readable format uses powers for 1024, but does not display the "i"
///    that is commonly used to denote Kibi, Mebi, etc.
/// 4. Kibi and Kilo are denoted differently ("k" and "K", respectively)
fn format_prefixed(prefixed: &NumberPrefix<f64>) -> String {
    match prefixed {
        NumberPrefix::Standalone(bytes) => bytes.to_string(),
        NumberPrefix::Prefixed(prefix, bytes) => {
            // Remove the "i" from "Ki", "Mi", etc. if present
            let prefix_str = prefix.symbol().trim_end_matches('i');

            // Check whether we get more than 10 if we round up to the first decimal
            // because we want do display 9.81 as "9.9", not as "10".
            if (10.0 * bytes).ceil() >= 100.0 {
                format!("{:.0}{prefix_str}", bytes.ceil())
            } else {
                format!("{:.1}{prefix_str}", (10.0 * bytes).ceil() / 10.0)
            }
        }
    }
}

pub fn human_readable(size: u64, sfmt: SizeFormat) -> String {
    match sfmt {
        SizeFormat::Binary => format_prefixed(&NumberPrefix::binary(size as f64)),
        SizeFormat::Decimal => format_prefixed(&NumberPrefix::decimal(size as f64)),
        SizeFormat::Bytes => size.to_string(),
    }
}

/// Get the thousands separator character from LC_NUMERIC locale.
///
/// Uses ICU to get the locale-appropriate grouping separator.
/// The result is cached after the first call for efficiency.
///
/// # Returns
/// - `'\0'` for C/POSIX locale (no separator)
/// - The locale's grouping separator character (e.g., ',' for en_US, '\u{202f}' for fr_FR)
pub fn get_thousands_separator() -> char {
    #[cfg(feature = "i18n-decimal")]
    {
        use crate::i18n::decimal::locale_grouping_separator;
        use crate::i18n::get_numeric_locale;

        // Check if this is C/POSIX locale (no thousands separator)
        let (locale, _) = get_numeric_locale();
        if locale.to_string() == "und" {
            return '\0';
        }

        // Get the grouping separator from ICU (cached via OnceLock)
        let sep = locale_grouping_separator();
        sep.chars().next().unwrap_or(',')
    }

    #[cfg(not(feature = "i18n-decimal"))]
    {
        // Fallback when i18n-decimal is not enabled: use comma
        ','
    }
}

/// Format a number with thousands separators based on LC_NUMERIC locale.
///
/// This function reads the LC_NUMERIC environment variable to determine
/// the thousands separator character. Falls back to comma if not set.
///
/// # Arguments
/// * `number` - The number to format
///
/// # Returns
/// A string with thousands separators inserted
///
/// # Examples
/// ```
/// use uucore::format::human::format_with_thousands_separator;
/// // With LC_NUMERIC=en_US.UTF-8 (or default)
/// assert_eq!(format_with_thousands_separator(1234567), "1,234,567");
/// // With LC_NUMERIC=de_DE.UTF-8
/// // assert_eq!(format_with_thousands_separator(1234567), "1.234.567");
/// ```
pub fn format_with_thousands_separator(number: u64) -> String {
    let separator = get_thousands_separator();

    // C/POSIX locale has no thousands separator
    if separator == '\0' {
        return number.to_string();
    }

    // Get locale-aware grouping sizes (primary, secondary)
    // Most locales: (3, 3), Indian locales: (3, 2)
    let (primary, secondary) = get_locale_grouping_sizes();

    let num_str = number.to_string();
    let len = num_str.len();

    // Calculate positions where separators should be inserted
    let mut sep_positions = Vec::new();
    let mut pos = len;

    // First group uses primary size
    if pos > primary as usize {
        pos -= primary as usize;
        sep_positions.push(pos);
    }

    // Subsequent groups use secondary size
    while pos > secondary as usize {
        pos -= secondary as usize;
        sep_positions.push(pos);
    }

    // Build result with separators
    let mut result = String::with_capacity(len + sep_positions.len());

    for (i, ch) in num_str.chars().enumerate() {
        if sep_positions.contains(&i) {
            result.push(separator);
        }
        result.push(ch);
    }

    result
}

/// Get locale-aware grouping sizes.
/// Returns (primary, secondary) group sizes.
/// Most locales return (3, 3), Indian locales return (3, 2).
#[cfg(feature = "i18n-decimal")]
fn get_locale_grouping_sizes() -> (u8, u8) {
    use crate::i18n::decimal::locale_grouping_sizes;
    *locale_grouping_sizes()
}

#[cfg(not(feature = "i18n-decimal"))]
fn get_locale_grouping_sizes() -> (u8, u8) {
    // Default to groups of 3 when i18n-decimal is not enabled
    (3, 3)
}

#[cfg(test)]
#[test]
fn test_human_readable() {
    let test_cases = [
        (133_456_345, SizeFormat::Binary, "128M"),
        (12 * 1024 * 1024, SizeFormat::Binary, "12M"),
        (8500, SizeFormat::Binary, "8.4K"),
    ];

    for &(size, sfmt, expected_str) in &test_cases {
        assert_eq!(human_readable(size, sfmt), expected_str);
    }
}
