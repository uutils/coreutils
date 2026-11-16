// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore newlocale uselocale freelocale fieldsets fset prefs febr abmon langinfo uppercased wcswidth alef Pagume

//! Locale-aware datetime formatting utilities using ICU and jiff-icu

use icu_calendar::Date;
use icu_calendar::cal::{Buddhist, Ethiopian, Gregorian, Iso, Persian};
use icu_datetime::DateTimeFormatter;
use icu_datetime::FixedCalendarDateTimeFormatter;
use icu_datetime::fieldsets;
use icu_locale::Locale;
use jiff::civil::Date as JiffDate;
use jiff_icu::ConvertFrom;
use std::sync::OnceLock;

use crate::i18n::get_locale_from_env;

/// Controls whether localized abbreviated month names are padded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamePadding {
    /// Raw names with no trailing padding, for `date` and similar utilities.
    Raw,
    /// Abbreviated month names padded to the widest one, for `ls` columns.
    Padded,
}

/// Get the locale for time/date formatting from `LC_TIME` environment variable
pub fn get_time_locale() -> &'static (Locale, super::UEncoding) {
    static TIME_LOCALE: OnceLock<(Locale, super::UEncoding)> = OnceLock::new();

    TIME_LOCALE.get_or_init(|| get_locale_from_env("LC_TIME"))
}

/// Check if we should use ICU for locale-aware time/date formatting
///
/// Returns true for non-C/POSIX locales, false otherwise
pub fn should_use_icu_locale() -> bool {
    use icu_locale::locale;

    // Cached: ls asks once per file.
    static USE_ICU: OnceLock<bool> = OnceLock::new();
    // Use ICU for non-default locales (anything other than C/POSIX)
    // The default locale is "und" (undefined) representing C/POSIX
    *USE_ICU.get_or_init(|| get_time_locale().0 != locale!("und"))
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

/// Create a formatter for `$calendar` only. `DateTimeFormatter::try_new` can
/// pick any calendar at runtime, so it links the data of all of them, which
/// every utility then pays for in relocations at startup.
macro_rules! new_formatter {
    ($fset:expr, $calendar:expr) => {
        FixedCalendarDateTimeFormatter::try_new((&get_time_locale().0).into(), $fset)
            .ok()
            .map(|f| f.into_formatter($calendar))
    };
}

/// Create a formatter for month names in the calendar that dates are converted
/// to, so that %B agrees with %m. Thai month names are the same in the Buddhist
/// and Gregorian calendars.
macro_rules! new_month_formatter {
    ($fset:expr) => {
        match get_locale_calendar_type(&get_time_locale().0) {
            CalendarType::Persian => new_formatter!($fset, Persian),
            CalendarType::Ethiopian => new_formatter!($fset, Ethiopian::new()),
            CalendarType::Buddhist | CalendarType::Gregorian => new_formatter!($fset, Gregorian),
        }
    };
}

thread_local! {
    // Not a `static OnceLock`: ICU formatters are neither `Send` nor `Sync`.
    static MONTH_LONG: Option<DateTimeFormatter<fieldsets::M>> =
        new_month_formatter!(fieldsets::M::long());
    static MONTH_MEDIUM: Option<DateTimeFormatter<fieldsets::M>> =
        new_month_formatter!(fieldsets::M::medium());
    // Weekday names do not depend on the calendar.
    static WEEKDAY_LONG: Option<DateTimeFormatter<fieldsets::E>> =
        new_formatter!(fieldsets::E::long(), Gregorian);
    static WEEKDAY_SHORT: Option<DateTimeFormatter<fieldsets::E>> =
        new_formatter!(fieldsets::E::short(), Gregorian);
}

macro_rules! format_with {
    ($formatter:ident, $date:expr) => {
        $formatter.with(|f| Some(f.as_ref()?.format($date).to_string()))
    };
}

/// ICU's medium month format may include trailing periods (e.g., "febr." for
/// Hungarian), which when combined with locale format strings that also add
/// periods after %b (e.g., "%Y. %b. %d") results in double periods ("febr..").
/// The standard C/POSIX locale via `nl_langinfo` returns abbreviations WITHOUT
/// trailing periods, so we strip them for consistency.
fn strip_abbrev_dots(name: &str) -> &str {
    name.trim_end_matches('.')
}

/// Sum of the per-character display widths of `s`.
///
/// Not `UnicodeWidthStr::width`, which counts the Arabic lam-alef sequence
/// as one cell where glibc's `wcswidth` (used by GNU ls) counts two.
fn display_width(s: &str) -> usize {
    use unicode_width::UnicodeWidthChar;
    s.chars().map(|c| c.width().unwrap_or(0)).sum()
}

/// Whether the `LC_TIME` locale writes its names in UTF-8, like ICU does.
///
/// Other codesets (`ru_RU.KOI8-R`, `zh_CN.GB18030`) keep the C names until
/// ICU names are transcoded: written as UTF-8, they would be mojibake. The
/// codeset comes from the locale itself, not from its name, since a locale
/// such as `fa_IR` is UTF-8 without saying so.
fn time_locale_is_utf8() -> bool {
    static IS_UTF8: OnceLock<bool> = OnceLock::new();
    *IS_UTF8.get_or_init(|| {
        let Some(name) = ["LC_ALL", "LC_TIME", "LANG"]
            .iter()
            .find_map(|&key| std::env::var(key).ok().filter(|l| !l.is_empty()))
        else {
            return true;
        };
        // A locale that is not installed: trust the codeset in its name.
        locale_codeset_is_utf8(&name).unwrap_or_else(|| {
            let name = name.split('@').next().unwrap_or_default();
            name.split_once('.')
                .is_none_or(|(_, codeset)| is_utf8_codeset(codeset.as_bytes()))
        })
    })
}

fn is_utf8_codeset(codeset: &[u8]) -> bool {
    codeset.eq_ignore_ascii_case(b"UTF-8") || codeset.eq_ignore_ascii_case(b"UTF8")
}

/// `nl_langinfo(CODESET)` of the locale `name`, `None` if it is not installed.
#[cfg(all(
    unix,
    not(target_os = "android"),
    not(target_os = "cygwin"),
    not(target_os = "netbsd"),
    not(target_os = "redox")
))]
fn locale_codeset_is_utf8(name: &str) -> Option<bool> {
    use core::ffi::CStr;
    use nix::libc;

    let name = std::ffi::CString::new(name).ok()?;
    // SAFETY: the locale object is only installed for this thread, while
    // `nl_langinfo` is read, and the previous one is restored before freeing it.
    unsafe {
        let loc = libc::newlocale(libc::LC_CTYPE_MASK, name.as_ptr(), std::ptr::null_mut());
        if loc.is_null() {
            return None;
        }
        let previous = libc::uselocale(loc);
        let codeset = libc::nl_langinfo(libc::CODESET);
        let is_utf8 = !codeset.is_null() && is_utf8_codeset(CStr::from_ptr(codeset).to_bytes());
        libc::uselocale(previous);
        libc::freelocale(loc);
        Some(is_utf8)
    }
}

/// Without `newlocale`, fall back to the codeset in the locale name.
#[cfg(not(all(
    unix,
    not(target_os = "android"),
    not(target_os = "cygwin"),
    not(target_os = "netbsd"),
    not(target_os = "redox")
)))]
fn locale_codeset_is_utf8(_name: &str) -> Option<bool> {
    None
}

thread_local! {
    // ICU falls back to placeholder names ("M01", "Mon") for a language it
    // has no data for: keep the C names then, like glibc does. Keep them too
    // when the locale is not UTF-8 (see `time_locale_is_utf8`).
    static HAS_NAMES: bool = time_locale_is_utf8()
        && Date::try_new_iso(2001, 1, 1)
            .ok()
            .and_then(|d| format_with!(MONTH_LONG, &d))
            .is_some_and(|name| name != "M01");
    // Widest abbreviated month name, only computed for padded output. One
    // date every five days reaches every month of every supported calendar,
    // including the five-day Ethiopian Pagume.
    static MONTH_MEDIUM_WIDTH: usize = MONTH_MEDIUM.with(|f| {
        f.as_ref().map_or(0, |f| {
            jiff::civil::date(2001, 1, 1)
                .series(jiff::Span::new().days(5))
                .take(74)
                .map(|d| f.format(&Date::<Iso>::convert_from(d)).to_string())
                .map(|name| display_width(strip_abbrev_dots(&name)))
                .max()
                .unwrap_or(0)
        })
    });
}

/// Pad an abbreviated month name to the width of the widest one. Like GNU ls,
/// a name starting with a digit (e.g. "3月") is right-aligned.
fn pad_month_abbrev(name: &str) -> String {
    let fill = MONTH_MEDIUM_WIDTH.with(|w| w.saturating_sub(display_width(name)));
    if name.starts_with(|c: char| c.is_ascii_digit()) {
        format!("{:fill$}{name}", "")
    } else {
        format!("{name}{:fill$}", "")
    }
}

/// Transform a strftime format string to use locale-specific calendar values.
/// With [`NamePadding::Padded`], `%b` is padded to a uniform width.
pub fn localize_format_string(format: &str, date: JiffDate, padding: NamePadding) -> String {
    type Entry = (String, JiffDate, NamePadding, String);
    thread_local! {
        // `ls -l` asks for the same date over and over: keep the last one.
        static LAST: std::cell::RefCell<Option<Entry>> = const { std::cell::RefCell::new(None) };
    }
    LAST.with_borrow_mut(|last| {
        if let Some((f, d, p, out)) = last
            && (f.as_str(), *d, *p) == (format, date, padding)
        {
            return out.clone();
        }
        let out = localize_format_string_uncached(format, date, padding);
        *last = Some((format.to_string(), date, padding, out.clone()));
        out
    })
}

fn localize_format_string_uncached(format: &str, date: JiffDate, padding: NamePadding) -> String {
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

    if !HAS_NAMES.with(|h| *h) {
        return fmt.replace(PERCENT_PLACEHOLDER, "%%");
    }

    // Format localized names using ICU DateTimeFormatter
    if fmt.contains("%B")
        && let Some(name) = format_with!(MONTH_LONG, &iso_date)
    {
        fmt = fmt.replace("%B", &name);
    }
    if (fmt.contains("%b") || fmt.contains("%h"))
        && let Some(name) = format_with!(MONTH_MEDIUM, &iso_date)
    {
        let month_abbrev = match padding {
            NamePadding::Raw => strip_abbrev_dots(&name).to_string(),
            NamePadding::Padded => pad_month_abbrev(strip_abbrev_dots(&name)),
        };
        fmt = fmt
            .replace("%b", &month_abbrev)
            .replace("%h", &month_abbrev);
    }
    if fmt.contains("%A")
        && let Some(name) = format_with!(WEEKDAY_LONG, &iso_date)
    {
        fmt = fmt.replace("%A", &name);
    }
    if fmt.contains("%a")
        && let Some(name) = format_with!(WEEKDAY_SHORT, &iso_date)
    {
        fmt = fmt.replace("%a", &name);
    }

    fmt.replace(PERCENT_PLACEHOLDER, "%%")
}

/// Abbreviated month names for the current `LC_TIME` locale as raw bytes,
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

/// Unix implementation using `nl_langinfo` for exact match with `locale abmon` output.
#[cfg(all(
    unix,
    not(target_os = "android"),
    not(target_os = "cygwin"),
    not(target_os = "redox")
))]
fn get_locale_months_inner() -> Option<[Vec<u8>; 12]> {
    use core::ffi::CStr;
    use nix::libc;

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

/// Non-Unix fallback using ICU `DateTimeFormatter`.
#[cfg(any(
    not(unix),
    target_os = "android",
    target_os = "cygwin",
    target_os = "redox"
))]
fn get_locale_months_inner() -> Option<[Vec<u8>; 12]> {
    // Abbreviated Gregorian names ("Jan", "Feb"), like nl_langinfo(ABMON_*) on
    // Unix: `sort -M` gives rank N to the name of ISO month N, whatever
    // calendar %B uses.
    let formatter = new_formatter!(fieldsets::M::medium(), Gregorian)?;

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

    /// Confirm that `M::short()` gives numeric months and `M::long()` gives full names,
    /// so `M::medium()` is the only correct choice for abbreviated month names.
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
