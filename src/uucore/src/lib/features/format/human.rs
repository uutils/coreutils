// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore gnulibs sfmt

//! `human`-size formatting
//!
//! Format sizes like gnulibs human_readable() would

use number_prefix::NumberPrefix;

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
/// This function reads the `LC_NUMERIC`, `LC_ALL`, or `LANG` environment
/// variables to determine the appropriate thousands separator character.
///
/// # Returns
/// - `'\0'` for C/POSIX locale (no separator)
/// - `'.'` for European locales (de_DE, fr_FR, it_IT, es_ES, etc.)
/// - `','` for other locales (default, en_US style)
fn get_thousands_separator() -> char {
    // Try to read LC_NUMERIC or LANG environment variable
    if let Ok(locale) = std::env::var("LC_NUMERIC")
        .or_else(|_| std::env::var("LC_ALL"))
        .or_else(|_| std::env::var("LANG"))
    {
        // C and POSIX locales have no thousands separator
        if locale == "C" || locale == "POSIX" || locale.starts_with("C.") {
            return '\0';
        }

        // Simple heuristic: European locales use period, others use comma
        // This covers common cases like de_DE, fr_FR, it_IT, es_ES, nl_NL, etc.
        if locale.starts_with("de_")
            || locale.starts_with("fr_")
            || locale.starts_with("it_")
            || locale.starts_with("es_")
            || locale.starts_with("nl_")
            || locale.starts_with("pt_")
            || locale.starts_with("da_")
            || locale.starts_with("sv_")
            || locale.starts_with("no_")
            || locale.starts_with("fi_")
        {
            return '.';
        }
    }

    // Default to comma (en_US style)
    ','
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
/// // Note: Output depends on LC_NUMERIC locale. This example assumes en_US.UTF-8
/// // To test with specific locale, set LC_NUMERIC environment variable before running tests
/// let result = format_with_thousands_separator(1234567);
/// // With en_US locale: "1,234,567"
/// // With de_DE locale: "1.234.567"
/// // With C/POSIX locale: "1234567"
/// assert!(!result.is_empty()); // Just verify it returns something
/// ```
pub fn format_with_thousands_separator(number: u64) -> String {
    const GROUPING_SIZE: usize = 3;

    let separator = get_thousands_separator();

    // C/POSIX locale has no thousands separator
    if separator == '\0' {
        return number.to_string();
    }

    let num_str = number.to_string();
    let len = num_str.len();

    // Numbers less than 1000 don't need separators
    if len <= GROUPING_SIZE {
        return num_str;
    }

    let mut result = String::with_capacity(len + (len - 1) / GROUPING_SIZE);

    for (i, ch) in num_str.chars().enumerate() {
        if i > 0 && (len - i) % GROUPING_SIZE == 0 {
            result.push(separator);
        }
        result.push(ch);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

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

    #[test]
    #[serial]
    fn test_format_with_thousands_separator() {
        // Save original locale variables
        let original_lc_numeric = std::env::var("LC_NUMERIC").ok();
        let original_lc_all = std::env::var("LC_ALL").ok();
        let original_lang = std::env::var("LANG").ok();

        // Test basic formatting with en_US locale (comma separator)
        // We explicitly set en_US and clear other locale vars to ensure consistent behavior
        unsafe {
            std::env::remove_var("LC_ALL");
            std::env::remove_var("LANG");
            std::env::set_var("LC_NUMERIC", "en_US.UTF-8");
        }

        assert_eq!(format_with_thousands_separator(0), "0");
        assert_eq!(format_with_thousands_separator(1), "1");
        assert_eq!(format_with_thousands_separator(12), "12");
        assert_eq!(format_with_thousands_separator(123), "123");
        assert_eq!(format_with_thousands_separator(1234), "1,234");
        assert_eq!(format_with_thousands_separator(12345), "12,345");
        assert_eq!(format_with_thousands_separator(123456), "123,456");
        assert_eq!(format_with_thousands_separator(1234567), "1,234,567");
        assert_eq!(format_with_thousands_separator(12345678), "12,345,678");
        assert_eq!(format_with_thousands_separator(123456789), "123,456,789");
        assert_eq!(format_with_thousands_separator(1234567890), "1,234,567,890");

        // Test large numbers
        assert_eq!(
            format_with_thousands_separator(u64::MAX),
            "18,446,744,073,709,551,615"
        );

        // Restore original locale variables
        unsafe {
            std::env::remove_var("LC_NUMERIC");
            std::env::remove_var("LC_ALL");
            std::env::remove_var("LANG");

            if let Some(locale) = original_lc_numeric {
                std::env::set_var("LC_NUMERIC", locale);
            }
            if let Some(locale) = original_lc_all {
                std::env::set_var("LC_ALL", locale);
            }
            if let Some(locale) = original_lang {
                std::env::set_var("LANG", locale);
            }
        }
    }

    #[test]
    #[serial]
    fn test_format_with_thousands_separator_locale() {
        // Save original locale variables
        let original_lc_numeric = std::env::var("LC_NUMERIC").ok();
        let original_lc_all = std::env::var("LC_ALL").ok();
        let original_lang = std::env::var("LANG").ok();

        unsafe {
            // Clear all locale vars first to ensure clean state
            std::env::remove_var("LC_ALL");
            std::env::remove_var("LANG");

            // Test with German locale (uses period as separator)
            std::env::set_var("LC_NUMERIC", "de_DE.UTF-8");
            assert_eq!(format_with_thousands_separator(1234567), "1.234.567");

            // Test with French locale (uses period as separator)
            std::env::set_var("LC_NUMERIC", "fr_FR.UTF-8");
            assert_eq!(format_with_thousands_separator(1234567), "1.234.567");

            // Test with US locale (uses comma as separator)
            std::env::set_var("LC_NUMERIC", "en_US.UTF-8");
            assert_eq!(format_with_thousands_separator(1234567), "1,234,567");

            // Test with C locale (no separator)
            std::env::set_var("LC_NUMERIC", "C");
            assert_eq!(format_with_thousands_separator(1234567), "1234567");

            // Test with POSIX locale (no separator)
            std::env::set_var("LC_NUMERIC", "POSIX");
            assert_eq!(format_with_thousands_separator(1234567), "1234567");

            // Restore original locale variables
            std::env::remove_var("LC_NUMERIC");
            std::env::remove_var("LC_ALL");
            std::env::remove_var("LANG");

            if let Some(locale) = original_lc_numeric {
                std::env::set_var("LC_NUMERIC", locale);
            }
            if let Some(locale) = original_lc_all {
                std::env::set_var("LC_ALL", locale);
            }
            if let Some(locale) = original_lang {
                std::env::set_var("LANG", locale);
            }
        }
    }

    #[test]
    fn test_get_thousands_separator() {
        // Save original locale
        let original_locale = std::env::var("LC_NUMERIC").ok();

        unsafe {
            // Test default (no locale set)
            std::env::remove_var("LC_NUMERIC");
            std::env::remove_var("LC_ALL");
            std::env::remove_var("LANG");
            assert_eq!(get_thousands_separator(), ',');

            // Test C locale
            std::env::set_var("LC_NUMERIC", "C");
            assert_eq!(get_thousands_separator(), '\0');

            // Test POSIX locale
            std::env::set_var("LC_NUMERIC", "POSIX");
            assert_eq!(get_thousands_separator(), '\0');

            // Test European locales
            std::env::set_var("LC_NUMERIC", "de_DE.UTF-8");
            assert_eq!(get_thousands_separator(), '.');

            std::env::set_var("LC_NUMERIC", "fr_FR.UTF-8");
            assert_eq!(get_thousands_separator(), '.');

            // Test US locale
            std::env::set_var("LC_NUMERIC", "en_US.UTF-8");
            assert_eq!(get_thousands_separator(), ',');

            // Restore original locale
            std::env::remove_var("LC_NUMERIC");
            if let Some(locale) = original_locale {
                std::env::set_var("LC_NUMERIC", locale);
            }
        }
    }
}
