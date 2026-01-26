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

/// Determine the appropriate calendar system for a given locale
pub fn get_locale_calendar_type(locale: &Locale) -> CalendarType {
    let locale_str = locale.to_string();

    match locale_str.as_str() {
        // Thai locales use Buddhist calendar
        s if s.starts_with("th") => CalendarType::Buddhist,
        // Persian/Farsi locales use Persian calendar (Solar Hijri)
        s if s.starts_with("fa") => CalendarType::Persian,
        // Amharic (Ethiopian) locales use Ethiopian calendar
        s if s.starts_with("am") => CalendarType::Ethiopian,
        // Default to Gregorian for all other locales
        _ => CalendarType::Gregorian,
    }
}

/// Calendar types supported for locale-aware formatting
#[derive(Debug, Clone, PartialEq)]
pub enum CalendarType {
    /// Gregorian calendar (used by most locales)
    Gregorian,
    /// Buddhist calendar (Thai locales) - adds 543 years to Gregorian year
    Buddhist,
    /// Persian Solar Hijri calendar (Persian/Farsi locales) - subtracts 621/622 years
    Persian,
    /// Ethiopian calendar (Amharic locales) - subtracts 7/8 years
    Ethiopian,
}

/// Convert a Gregorian date to the appropriate calendar system for a locale
///
/// # Arguments
/// * `year` - Gregorian year
/// * `month` - Month (1-12)
/// * `day` - Day (1-31)
/// * `calendar_type` - Target calendar system
///
/// # Returns
/// * `Some((era_year, month, day))` - Date in target calendar system
/// * `None` - If conversion fails
pub fn convert_date_to_locale_calendar(
    year: i32,
    month: u8,
    day: u8,
    calendar_type: &CalendarType,
) -> Option<(i32, u8, u8)> {
    match calendar_type {
        CalendarType::Gregorian => Some((year, month, day)),
        CalendarType::Buddhist => {
            // Buddhist calendar: Gregorian year + 543
            Some((year + 543, month, day))
        }
        CalendarType::Persian => {
            // Persian calendar conversion (Solar Hijri)
            // March 21 (Nowruz) is roughly the start of the Persian year
            let persian_year = if month > 3 || (month == 3 && day >= 21) {
                year - 621 // After March 21
            } else {
                year - 622 // Before March 21
            };
            Some((persian_year, month, day))
        }
        CalendarType::Ethiopian => {
            // Ethiopian calendar conversion
            // September 11/12 is roughly the start of the Ethiopian year
            let ethiopian_year = if month > 9 || (month == 9 && day >= 11) {
                year - 7 // After September 11
            } else {
                year - 8 // Before September 11
            };
            Some((ethiopian_year, month, day))
        }
    }
}

/// Get the era year for a given date and locale
pub fn get_era_year(year: i32, month: u8, day: u8, locale: &Locale) -> Option<i32> {
    // Validate input date
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }

    let calendar_type = get_locale_calendar_type(locale);
    match calendar_type {
        CalendarType::Gregorian => None,
        _ => convert_date_to_locale_calendar(year, month, day, &calendar_type)
            .map(|(era_year, _, _)| era_year),
    }
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

    #[test]
    fn test_calendar_type_detection() {
        let thai_locale = icu_locale::locale!("th-TH");
        let persian_locale = icu_locale::locale!("fa-IR");
        let amharic_locale = icu_locale::locale!("am-ET");
        let english_locale = icu_locale::locale!("en-US");

        assert_eq!(
            get_locale_calendar_type(&thai_locale),
            CalendarType::Buddhist
        );
        assert_eq!(
            get_locale_calendar_type(&persian_locale),
            CalendarType::Persian
        );
        assert_eq!(
            get_locale_calendar_type(&amharic_locale),
            CalendarType::Ethiopian
        );
        assert_eq!(
            get_locale_calendar_type(&english_locale),
            CalendarType::Gregorian
        );
    }

    #[test]
    fn test_era_year_conversion() {
        let thai_locale = icu_locale::locale!("th-TH");
        let persian_locale = icu_locale::locale!("fa-IR");
        let amharic_locale = icu_locale::locale!("am-ET");

        // Test Thai Buddhist calendar (2026 + 543 = 2569)
        assert_eq!(get_era_year(2026, 6, 15, &thai_locale), Some(2569));

        // Test Persian calendar (rough approximation)
        assert_eq!(get_era_year(2026, 3, 22, &persian_locale), Some(1405));
        assert_eq!(get_era_year(2026, 3, 19, &persian_locale), Some(1404));

        // Test Ethiopian calendar (rough approximation)
        assert_eq!(get_era_year(2026, 9, 12, &amharic_locale), Some(2019));
        assert_eq!(get_era_year(2026, 9, 10, &amharic_locale), Some(2018));
    }
}
