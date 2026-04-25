// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore fieldsets prefs febr abmon langinfo uppercased

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
                (
                    d.year().extended_year(),
                    d.month().ordinal,
                    d.day_of_month().0,
                )
            }
            CalendarType::Persian => {
                let d = iso_date.to_calendar(Persian);
                (
                    d.year().extended_year(),
                    d.month().ordinal,
                    d.day_of_month().0,
                )
            }
            CalendarType::Ethiopian => {
                let d = iso_date.to_calendar(Ethiopian::new());
                (
                    d.year().extended_year(),
                    d.month().ordinal,
                    d.day_of_month().0,
                )
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

    fmt.replace(PERCENT_PLACEHOLDER, "%%")
}

/// Abbreviated month names for the current LC_TIME locale as raw bytes,
/// with blanks stripped and uppercased using ASCII case folding.
///
/// Each entry corresponds to months January (index 0) through December (index 11).
/// Returns `None` for C/POSIX locale (caller should use English defaults).
/// This matches the GNU coreutils approach of storing uppercased, blank-stripped names.
pub fn get_locale_months() -> Option<&'static [Vec<u8>; 12]> {
    static LOCALE_MONTHS: OnceLock<Option<[Vec<u8>; 12]>> = OnceLock::new();

    LOCALE_MONTHS
        .get_or_init(|| {
            if !should_use_icu_locale() {
                return None;
            }
            get_locale_months_inner()
        })
        .as_ref()
}

/// Unix implementation using nl_langinfo for exact match with `locale abmon` output.
#[cfg(all(unix, not(target_os = "android"), not(target_os = "redox")))]
fn get_locale_months_inner() -> Option<[Vec<u8>; 12]> {
    use libc;
    use std::ffi::CStr;

    let abmon_items: [libc::nl_item; 12] = [
        libc::ABMON_1,
        libc::ABMON_2,
        libc::ABMON_3,
        libc::ABMON_4,
        libc::ABMON_5,
        libc::ABMON_6,
        libc::ABMON_7,
        libc::ABMON_8,
        libc::ABMON_9,
        libc::ABMON_10,
        libc::ABMON_11,
        libc::ABMON_12,
    ];

    // SAFETY: setlocale and nl_langinfo are standard POSIX functions.
    // We call setlocale(LC_TIME, "") to initialize from environment variables,
    // then read the abbreviated month names. This is called once (via OnceLock)
    // and cached, so the race window with other setlocale callers is minimal.
    // The nl_langinfo return pointer is immediately copied below.
    unsafe {
        libc::setlocale(libc::LC_TIME, c"".as_ptr());
    }

    let mut months: [Vec<u8>; 12] = Default::default();
    for (i, &item) in abmon_items.iter().enumerate() {
        // SAFETY: nl_langinfo returns a valid C string pointer for valid nl_item values.
        let ptr = unsafe { libc::nl_langinfo(item) };
        if ptr.is_null() {
            return None;
        }
        let name = unsafe { CStr::from_ptr(ptr) }.to_bytes();
        if name.is_empty() {
            return None;
        }
        // Strip blanks and uppercase using ASCII case folding, matching GNU behavior
        months[i] = name
            .iter()
            .filter(|&&b| !b.is_ascii_whitespace())
            .map(|&b| b.to_ascii_uppercase())
            .collect();
    }

    Some(months)
}

/// Non-Unix fallback using ICU DateTimeFormatter.
#[cfg(any(not(unix), target_os = "android", target_os = "redox"))]
fn get_locale_months_inner() -> Option<[Vec<u8>; 12]> {
    let (locale, _) = get_time_locale();
    let locale_prefs = locale.clone().into();
    // M::medium() produces abbreviated month names (e.g. "Jan", "Feb") matching
    // nl_langinfo(ABMON_*) on Unix. M::short() produces numeric ("1", "2") and
    // M::long() produces full names ("January", "February").
    let formatter = DateTimeFormatter::try_new(locale_prefs, fieldsets::M::medium()).ok()?;

    let mut months: [Vec<u8>; 12] = Default::default();
    for i in 0..12u8 {
        let iso_date = Date::<Iso>::try_new_iso(2000, i + 1, 1).ok()?;
        let formatted = formatter.format(&iso_date).to_string();
        // Strip blanks and uppercase using ASCII case folding
        months[i as usize] = formatted
            .bytes()
            .filter(|b| !b.is_ascii_whitespace())
            .map(|b| b.to_ascii_uppercase())
            .collect();
    }

    Some(months)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify that ICU `M::medium()` produces abbreviated month names matching
    /// what `nl_langinfo(ABMON_*)` returns on Unix. This is the format used by
    /// the non-Unix fallback in `get_locale_months_inner`.
    #[test]
    fn test_icu_medium_month_produces_abbreviated_names() {
        use icu_locale::locale;

        let locale: Locale = locale!("en-US");
        let formatter = DateTimeFormatter::try_new(locale.into(), fieldsets::M::medium()).unwrap();

        let expected = [
            "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
        ];

        for (i, exp) in expected.iter().enumerate() {
            let iso_date = Date::<Iso>::try_new_iso(2000, (i + 1) as u8, 1).unwrap();
            let formatted = formatter.format(&iso_date).to_string();
            assert_eq!(
                &formatted,
                exp,
                "M::medium() for month {} should produce abbreviated name",
                i + 1
            );
        }
    }

    /// Confirm that M::short() gives numeric months and M::long() gives full names,
    /// so M::medium() is the only correct choice for abbreviated month names.
    #[test]
    fn test_icu_short_and_long_month_formats_differ() {
        use icu_locale::locale;

        let locale: Locale = locale!("en-US");
        let iso_jan = Date::<Iso>::try_new_iso(2000, 1, 1).unwrap();

        let short_fmt =
            DateTimeFormatter::try_new(locale.clone().into(), fieldsets::M::short()).unwrap();
        let long_fmt = DateTimeFormatter::try_new(locale.into(), fieldsets::M::long()).unwrap();

        // M::short() produces numeric ("1"), not "Jan"
        assert_eq!(short_fmt.format(&iso_jan).to_string(), "1");
        // M::long() produces full name ("January"), not "Jan"
        assert_eq!(long_fmt.format(&iso_jan).to_string(), "January");
    }

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
