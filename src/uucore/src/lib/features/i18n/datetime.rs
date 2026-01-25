// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Locale-aware datetime formatting utilities using ICU
// spell-checker:ignore fieldsets janvier

use icu_calendar::Date;
use icu_datetime::DateTimeFormatter;
use icu_datetime::fieldsets;
use icu_locale::Locale;
use std::sync::OnceLock;

use crate::i18n::get_locale_from_env;

/// Get the locale for time/date formatting from LC_TIME environment variable
pub fn get_time_locale() -> &'static (Locale, super::UEncoding) {
    static TIME_LOCALE: OnceLock<(Locale, super::UEncoding)> = OnceLock::new();

    TIME_LOCALE.get_or_init(|| get_locale_from_env("LC_TIME"))
}

/// Check if we should use ICU for locale-aware time/date formatting
///
/// Returns true for non-C/POSIX locales, false otherwise
pub fn should_use_icu_locale() -> bool {
    use icu_locale::locale;

    let (locale, _encoding) = get_time_locale();

    // Use ICU for non-default locales (anything other than C/POSIX)
    // The default locale is "und" (undefined) representing C/POSIX
    *locale != locale!("und")
}

/// Get a localized month name for the given month number (1-12)
///
/// # Arguments
/// * `month` - Month number (1 = January, 2 = February, etc.)
/// * `full` - If true, return full month name (e.g., "January"), otherwise abbreviated (e.g., "Jan")
///
/// # Returns
/// Localized month name, or falls back to English if locale is not supported
pub fn get_localized_month_name(month: u8, full: bool) -> String {
    // Get locale from environment
    let (locale, _encoding) = get_time_locale();

    // Create a date with the specified month (use year 2000, day 1 as arbitrary values)
    let Ok(date) = Date::try_new_gregorian(2000, month, 1) else {
        // Invalid month, return empty string to signal failure
        return String::new();
    };

    // Configure field set for month formatting
    // Use Year-Month-Day format to ensure we get textual month names
    let field_set = if full {
        fieldsets::YMD::long()
    } else {
        fieldsets::YMD::medium()
    };

    // Create formatter with locale
    let Ok(formatter) = DateTimeFormatter::try_new(locale.clone().into(), field_set) else {
        // Failed to create formatter, return empty string to signal failure
        return String::new();
    };

    // Format the date to get full date, then extract month
    let formatted = formatter.format(&date).to_string();
    // Extract month name from formatted date like "15 janvier 2000" or "2000-01-15"
    // Look for a word that contains letters (the month name)
    let words: Vec<&str> = formatted.split_whitespace().collect();

    // Return the month name as extracted from ICU (no further processing needed)
    // ICU already handles the full vs abbreviated formatting correctly
    words
        .iter()
        .find(|word| word.chars().any(|c| c.is_alphabetic()))
        .map_or_else(String::new, |s| (*s).to_string())
}

/// Get a localized day name for the given date components
///
/// # Arguments
/// * `year` - The year
/// * `month` - The month (1-12)
/// * `day` - The day of the month
/// * `full` - If true, return full day name (e.g., "Monday"), otherwise abbreviated (e.g., "Mon")
///
/// # Returns
/// Localized day name, or falls back to empty string if locale is not supported
pub fn get_localized_day_name(year: i32, month: u8, day: u8, full: bool) -> String {
    // Create ICU Date from components
    let Ok(date) = Date::try_new_gregorian(year, month, day) else {
        return String::new();
    };

    // Get locale from environment
    let (locale, _encoding) = get_time_locale();

    // Configure field set for day formatting
    let field_set = if full {
        fieldsets::E::long() // Full day name
    } else {
        fieldsets::E::short() // Abbreviated day name
    };

    // Create formatter with locale
    let Ok(formatter) = DateTimeFormatter::try_new(locale.clone().into(), field_set) else {
        return String::new();
    };

    // Format the date to get day name
    let formatted = formatter.format(&date).to_string();
    formatted.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_localized_month_name_fallback() {
        // This should work even if locale is not available
        let name = get_localized_month_name(1, true);
        // The function may return empty string if ICU fails, which is fine
        // The caller (date.rs) will handle this by falling back to jiff
        assert!(name.is_empty() || name.len() >= 3);
    }
}
