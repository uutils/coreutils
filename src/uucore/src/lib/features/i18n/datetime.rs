// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore fieldsets prefs febr abmon langinfo uppercased

//! Locale-aware datetime formatting utilities using ICU and jiff-icu

use icu_calendar::Date;
use icu_calendar::cal::{Buddhist, Ethiopian, Iso, Persian};
use icu_locale::Locale;
use jiff::civil::Date as JiffDate;
use jiff_icu::ConvertFrom;
use std::sync::OnceLock;

#[cfg(any(
    not(unix),
    target_os = "android",
    target_os = "cygwin",
    target_os = "redox"
))]
use icu_datetime::DateTimeFormatter;
#[cfg(any(
    not(unix),
    target_os = "android",
    target_os = "cygwin",
    target_os = "redox"
))]
use icu_datetime::fieldsets;
#[cfg(all(
    unix,
    not(target_os = "android"),
    not(target_os = "cygwin"),
    not(target_os = "redox")
))]
use nix::libc;

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

/// Locale-specific month name for the current `LC_TIME` locale.
#[cfg(all(
    unix,
    not(target_os = "android"),
    not(target_os = "cygwin"),
    not(target_os = "redox")
))]
fn locale_month_name(date: &Date<Iso>, long: bool) -> Option<String> {
    use std::ffi::CStr;

    let month_items: [libc::nl_item; 12] = if long {
        [
            libc::MON_1,
            libc::MON_2,
            libc::MON_3,
            libc::MON_4,
            libc::MON_5,
            libc::MON_6,
            libc::MON_7,
            libc::MON_8,
            libc::MON_9,
            libc::MON_10,
            libc::MON_11,
            libc::MON_12,
        ]
    } else {
        [
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
        ]
    };

    unsafe {
        libc::setlocale(libc::LC_TIME, c"".as_ptr());
    }

    let ordinal = usize::from(date.month().ordinal).checked_sub(1)?;
    let ptr = unsafe { libc::nl_langinfo(month_items[ordinal]) };
    if ptr.is_null() {
        return None;
    }

    let name = unsafe { CStr::from_ptr(ptr) }.to_string_lossy();
    if name.is_empty() {
        None
    } else {
        Some(name.into_owned())
    }
}

/// Locale-specific weekday name for the current `LC_TIME` locale.
#[cfg(all(
    unix,
    not(target_os = "android"),
    not(target_os = "cygwin"),
    not(target_os = "redox")
))]
fn locale_weekday_name(date: &Date<Iso>, long: bool) -> Option<String> {
    use std::ffi::CStr;

    let weekday_items: [libc::nl_item; 7] = if long {
        [
            libc::DAY_1,
            libc::DAY_2,
            libc::DAY_3,
            libc::DAY_4,
            libc::DAY_5,
            libc::DAY_6,
            libc::DAY_7,
        ]
    } else {
        [
            libc::ABDAY_1,
            libc::ABDAY_2,
            libc::ABDAY_3,
            libc::ABDAY_4,
            libc::ABDAY_5,
            libc::ABDAY_6,
            libc::ABDAY_7,
        ]
    };

    unsafe {
        libc::setlocale(libc::LC_TIME, c"".as_ptr());
    }

    let weekday = usize::from((date.weekday() as u8) % 7);
    let ptr = unsafe { libc::nl_langinfo(weekday_items[weekday]) };
    if ptr.is_null() {
        return None;
    }

    let name = unsafe { CStr::from_ptr(ptr) }.to_string_lossy();
    if name.is_empty() {
        None
    } else {
        Some(name.into_owned())
    }
}

/// Locale-specific month name for the current `LC_TIME` locale.
#[cfg(any(
    not(unix),
    target_os = "android",
    target_os = "cygwin",
    target_os = "redox"
))]
fn locale_month_name(date: &Date<Iso>, long: bool) -> Option<String> {
    let (locale, _) = get_time_locale();
    let locale = if locale.to_string().starts_with("th") {
        icu_locale::locale!("en-US")
    } else {
        locale.clone()
    };
    let locale_prefs = locale.into();
    let formatter = DateTimeFormatter::try_new(
        locale_prefs,
        if long {
            fieldsets::M::long()
        } else {
            fieldsets::M::medium()
        },
    )
    .ok()?;

    let name = formatter.format(date).to_string();
    Some(if long {
        name
    } else {
        name.trim_end_matches('.').to_string()
    })
}

/// Locale-specific weekday name for the current `LC_TIME` locale.
#[cfg(any(
    not(unix),
    target_os = "android",
    target_os = "cygwin",
    target_os = "redox"
))]
fn locale_weekday_name(date: &Date<Iso>, long: bool) -> Option<String> {
    let (locale, _) = get_time_locale();
    let locale = if locale.to_string().starts_with("th") {
        icu_locale::locale!("en-US")
    } else {
        locale.clone()
    };
    let locale_prefs = locale.into();
    let formatter = DateTimeFormatter::try_new(
        locale_prefs,
        if long {
            fieldsets::E::long()
        } else {
            fieldsets::E::short()
        },
    )
    .ok()?;

    Some(formatter.format(date).to_string())
}

/// Transform a strftime format string to use locale-specific calendar values
pub fn localize_format_string(format: &str, date: JiffDate) -> String {
    const PERCENT_PLACEHOLDER: &str = "\x00\x00";

    let (locale, _) = get_time_locale();
    let iso_date = Date::<Iso>::convert_from(date);

    let mut fmt = format.replace("%%", PERCENT_PLACEHOLDER);
    // Leave `%EY` untouched so GNU-compatible alternate year formatting can be
    // handled by the underlying strftime implementation.
    let calendar_type = get_locale_calendar_type(locale);
    match calendar_type {
        CalendarType::Buddhist => {
            let d = iso_date.to_calendar(Buddhist);
            let buddhist_year = d.year().era_year_or_related_iso();
            fmt = fmt
                .replace("%EY", &format!("พ.ศ. {buddhist_year}"))
                .replace("%EC", "พ.ศ.")
                .replace("%Ey", &buddhist_year.to_string());
        }
        CalendarType::Persian => {
            let d = iso_date.to_calendar(Persian);
            let cal_year = d.year().extended_year();
            let cal_month = d.month().ordinal;
            let cal_day = d.day_of_month().0;
            fmt = fmt
                .replace("%Y", &cal_year.to_string())
                .replace("%m", &format!("{cal_month:02}"))
                .replace("%d", &format!("{cal_day:02}"))
                .replace("%e", &format!("{cal_day:2}"));
        }
        CalendarType::Ethiopian => {
            let d = iso_date.to_calendar(Ethiopian::new());
            let cal_year = d.year().extended_year();
            let cal_month = d.month().ordinal;
            let cal_day = d.day_of_month().0;
            fmt = fmt
                .replace("%Y", &cal_year.to_string())
                .replace("%m", &format!("{cal_month:02}"))
                .replace("%d", &format!("{cal_day:02}"))
                .replace("%e", &format!("{cal_day:2}"));
        }
        CalendarType::Gregorian => {}
    }

    if fmt.contains("%B") {
        if let Some(month_name) = locale_month_name(&iso_date, true) {
            fmt = fmt.replace("%B", &month_name);
        }
    }
    if fmt.contains("%b") || fmt.contains("%h") {
        if let Some(month_name) = locale_month_name(&iso_date, false) {
            fmt = fmt.replace("%b", &month_name).replace("%h", &month_name);
        }
    }
    if fmt.contains("%A") {
        if let Some(weekday_name) = locale_weekday_name(&iso_date, true) {
            fmt = fmt.replace("%A", &weekday_name);
        }
    }
    if fmt.contains("%a") {
        if let Some(weekday_name) = locale_weekday_name(&iso_date, false) {
            fmt = fmt.replace("%a", &weekday_name);
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
#[cfg(all(
    unix,
    not(target_os = "android"),
    not(target_os = "cygwin"),
    not(target_os = "redox")
))]
fn get_locale_months_inner() -> Option<[Vec<u8>; 12]> {
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
#[cfg(any(
    not(unix),
    target_os = "android",
    target_os = "cygwin",
    target_os = "redox"
))]
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
