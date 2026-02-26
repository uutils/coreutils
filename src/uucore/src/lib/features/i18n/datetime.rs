// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore fieldsets prefs febr

//! Locale-aware datetime formatting utilities using ICU and jiff-icu

use icu_calendar::Date;
use icu_calendar::cal::{Buddhist, Ethiopian, Iso, Persian};
use icu_datetime::DateTimeFormatter;
use icu_datetime::fieldsets;
use icu_locale::Locale;
use jiff::civil::Date as JiffDate;
use jiff_icu::ConvertFrom;
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

/// Transform a strftime format string to use locale-specific calendar values
pub fn localize_format_string(format: &str, date: JiffDate) -> String {
    const PERCENT_PLACEHOLDER: &str = "\x00\x00";

    let (locale, _) = get_time_locale();
    let iso_date = Date::<Iso>::convert_from(date);

    let mut fmt = format.replace("%%", PERCENT_PLACEHOLDER);

    // For non-Gregorian calendars, replace date components with converted values
    let calendar_type = get_locale_calendar_type(locale);
    if calendar_type != CalendarType::Gregorian {
        let (cal_year, cal_month, cal_day) = match calendar_type {
            CalendarType::Buddhist => {
                let d = iso_date.to_calendar(Buddhist);
                (d.extended_year(), d.month().ordinal, d.day_of_month().0)
            }
            CalendarType::Persian => {
                let d = iso_date.to_calendar(Persian);
                (d.extended_year(), d.month().ordinal, d.day_of_month().0)
            }
            CalendarType::Ethiopian => {
                let d = iso_date.to_calendar(Ethiopian::new());
                (d.extended_year(), d.month().ordinal, d.day_of_month().0)
            }
            CalendarType::Gregorian => unreachable!(),
        };
        fmt = fmt
            .replace("%Y", &cal_year.to_string())
            .replace("%m", &format!("{cal_month:02}"))
            .replace("%d", &format!("{cal_day:02}"))
            .replace("%e", &format!("{cal_day:2}"));
    }

    // Format localized names using ICU DateTimeFormatter
    let locale_prefs = locale.clone().into();

    if fmt.contains("%B") {
        if let Ok(f) = DateTimeFormatter::try_new(locale_prefs, fieldsets::M::long()) {
            fmt = fmt.replace("%B", &f.format(&iso_date).to_string());
        }
    }
    if fmt.contains("%b") || fmt.contains("%h") {
        if let Ok(f) = DateTimeFormatter::try_new(locale_prefs, fieldsets::M::medium()) {
            // ICU's medium format may include trailing periods (e.g., "febr." for Hungarian),
            // which when combined with locale format strings that also add periods after
            // %b (e.g., "%Y. %b. %d") results in double periods ("febr..").
            // The standard C/POSIX locale via nl_langinfo returns abbreviations
            // WITHOUT trailing periods, so we strip them here for consistency.
            let month_abbrev = f.format(&iso_date).to_string();
            let month_abbrev = month_abbrev.trim_end_matches('.').to_string();
            fmt = fmt
                .replace("%b", &month_abbrev)
                .replace("%h", &month_abbrev);
        }
    }
    if fmt.contains("%A") {
        if let Ok(f) = DateTimeFormatter::try_new(locale_prefs, fieldsets::E::long()) {
            fmt = fmt.replace("%A", &f.format(&iso_date).to_string());
        }
    }
    if fmt.contains("%a") {
        if let Ok(f) = DateTimeFormatter::try_new(locale_prefs, fieldsets::E::short()) {
            fmt = fmt.replace("%a", &f.format(&iso_date).to_string());
        }
    }

    // Handle E and O modifiers (POSIX locale extensions)
    // These request alternative representations (e.g., era names, alternative numerals)
    fmt = handle_eo_modifiers(&fmt, iso_date, locale);

    fmt.replace(PERCENT_PLACEHOLDER, "%%")
}

/// Handle E and O modifiers for alternative locale-specific representations.
///
/// E modifiers request alternative representations (e.g., era names, alternative date formats).
/// O modifiers request alternative numeric symbols (e.g., Arabic-Indic digits, Eastern Arabic numerals).
fn handle_eo_modifiers(fmt: &str, iso_date: Date<Iso>, locale: &Locale) -> String {
    let mut result = fmt.to_string();
    let locale_prefs = locale.clone().into();

    // Handle %OB - Alternative month names (standalone format)
    // This is used when the month name appears without a day (e.g., "June" vs "June 1st")
    if result.contains("%OB") {
        // For now, treat %OB the same as %B since ICU doesn't have a direct standalone variant
        if let Ok(f) = DateTimeFormatter::try_new(locale_prefs, fieldsets::M::long()) {
            result = result.replace("%OB", &f.format(&iso_date).to_string());
        }
    }

    // Handle simple E modifiers without other flags: %EY, %Ey, %EC, %EB
    // Process these first before the more complex patterns
    for (pattern, _replacement) in [
        ("%EY", iso_date.extended_year().to_string()),
        ("%Ey", format!("{:02}", iso_date.extended_year() % 100)),
        ("%EC", format!("{:02}", iso_date.extended_year() / 100)),
    ] {
        if result.contains(pattern) {
            // For non-Gregorian calendars, use the extended year as alternative representation
            let calendar_type = get_locale_calendar_type(locale);
            let alt_year = if calendar_type == CalendarType::Gregorian {
                iso_date.extended_year()
            } else {
                match calendar_type {
                    CalendarType::Buddhist => {
                        let d = iso_date.to_calendar(Buddhist);
                        d.extended_year()
                    }
                    CalendarType::Persian => {
                        let d = iso_date.to_calendar(Persian);
                        d.extended_year()
                    }
                    CalendarType::Ethiopian => {
                        let d = iso_date.to_calendar(Ethiopian::new());
                        d.extended_year()
                    }
                    CalendarType::Gregorian => unreachable!(),
                }
            };

            let value = match pattern {
                "%EY" => alt_year.to_string(),
                "%Ey" => format!("{:02}", alt_year % 100),
                "%EC" => format!("{:02}", alt_year / 100),
                _ => unreachable!(),
            };
            result = result.replace(pattern, &value);
        }
    }

    // Handle O modifiers for alternative numeric symbols
    // These are locale-specific and typically use native numeral systems
    // For now, we fall back to standard formatting since full O modifier support
    // requires ICU's FixedDecimalFormatter with locale-specific numeral systems
    let o_specifiers = [
        ("%Od", "d"),
        ("%Oe", "e"),
        ("%OH", "H"),
        ("%OI", "I"),
        ("%Om", "m"),
        ("%OM", "M"),
        ("%OS", "S"),
        ("%Ou", "u"),
        ("%OU", "U"),
        ("%OV", "V"),
        ("%Ow", "w"),
        ("%OW", "W"),
        ("%Oy", "y"),
    ];

    for (o_spec, base_spec) in o_specifiers {
        if result.contains(o_spec) {
            // Convert O modifier to base specifier for jiff to handle
            // Full O modifier support would use ICU's FixedDecimalFormatter
            // with the locale's default numeral system
            result = result.replace(o_spec, &format!("%{base_spec}"));
        }
    }

    result
}

/// Check if format string contains E or O locale modifiers.
///
/// This is a simple check that looks for the presence of %E or %O patterns.
/// It handles both simple modifiers (%EY, %Od) and modifiers with flags/width (%_10EY).
pub fn has_locale_modifiers(format: &str) -> bool {
    // Simple check for %E or %O patterns
    // Note: This is a quick check that may have false positives for %%E or similar,
    // but that's acceptable for our use case
    format.contains("%E") || format.contains("%O")
}

/// Transform a strftime format string with E/O modifiers to use locale-specific values.
///
/// This function processes E/O modifiers and returns a tuple of:
/// - The transformed format string with E/O modifiers replaced by their values
/// - A flag indicating whether E/O modifiers were found and processed
///
/// This is used by the date command to handle POSIX locale extensions.
pub fn localize_format_string_with_modifiers(format: &str, date: JiffDate) -> (String, bool) {
    const PERCENT_PLACEHOLDER: &str = "\x00\x00";

    let (locale, _) = get_time_locale();

    // Check if format contains E or O modifiers
    let has_eo_modifiers = has_locale_modifiers(format);

    if !has_eo_modifiers {
        // No E/O modifiers, use standard localization
        return (localize_format_string(format, date), false);
    }

    let iso_date = Date::<Iso>::convert_from(date);
    let mut fmt = format.replace("%%", PERCENT_PLACEHOLDER);

    // Process E and O modifiers
    fmt = handle_eo_modifiers(&fmt, iso_date, locale);

    // Apply standard localization for remaining specifiers
    fmt = localize_format_string(&fmt, date);

    (fmt.replace(PERCENT_PLACEHOLDER, "%%"), true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calendar_type_detection() {
        use icu_locale::locale;
        assert_eq!(
            get_locale_calendar_type(&locale!("th-TH")),
            CalendarType::Buddhist
        );
        assert_eq!(
            get_locale_calendar_type(&locale!("fa-IR")),
            CalendarType::Persian
        );
        assert_eq!(
            get_locale_calendar_type(&locale!("am-ET")),
            CalendarType::Ethiopian
        );
        assert_eq!(
            get_locale_calendar_type(&locale!("en-US")),
            CalendarType::Gregorian
        );
    }
}
