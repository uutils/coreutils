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

thread_local! {
    // Not a `static OnceLock`: ICU formatters are neither `Send` nor `Sync`.
    static MONTH_LONG: Option<DateTimeFormatter<fieldsets::M>> =
        DateTimeFormatter::try_new(get_time_locale().0.clone().into(), fieldsets::M::long()).ok();
    static MONTH_MEDIUM: Option<DateTimeFormatter<fieldsets::M>> =
        DateTimeFormatter::try_new(get_time_locale().0.clone().into(), fieldsets::M::medium()).ok();
    static WEEKDAY_LONG: Option<DateTimeFormatter<fieldsets::E>> =
        DateTimeFormatter::try_new(get_time_locale().0.clone().into(), fieldsets::E::long()).ok();
    static WEEKDAY_SHORT: Option<DateTimeFormatter<fieldsets::E>> =
        DateTimeFormatter::try_new(get_time_locale().0.clone().into(), fieldsets::E::short()).ok();
}

macro_rules! format_with {
    ($formatter:ident, $date:expr) => {
        $formatter.with(|f| Some(f.as_ref()?.format($date).to_string()))
    };
}

/// A date as the locale shows it: in the locale's calendar, with localized
/// month and weekday names.
pub struct LocalizedDate {
    iso_date: Date<Iso>,
    /// Year, month and day in the locale's calendar, unless it is Gregorian
    converted: Option<(i32, u8, u8)>,
}

impl LocalizedDate {
    pub fn new(date: JiffDate) -> Self {
        let iso_date = Date::<Iso>::convert_from(date);
        let converted = match get_locale_calendar_type(&get_time_locale().0) {
            CalendarType::Gregorian => None,
            CalendarType::Buddhist => {
                let d = iso_date.to_calendar(Buddhist);
                Some((
                    d.year().extended_year(),
                    d.month().ordinal,
                    d.day_of_month().0,
                ))
            }
            CalendarType::Persian => {
                let d = iso_date.to_calendar(Persian);
                Some((
                    d.year().extended_year(),
                    d.month().ordinal,
                    d.day_of_month().0,
                ))
            }
            CalendarType::Ethiopian => {
                let d = iso_date.to_calendar(Ethiopian::new());
                Some((
                    d.year().extended_year(),
                    d.month().ordinal,
                    d.day_of_month().0,
                ))
            }
        };
        Self {
            iso_date,
            converted,
        }
    }

    /// The value of the strftime conversion `conv` for this date, as it is
    /// formatted without flags or width, or `None` if the locale does not
    /// change it. Other fields, like `%y` or `%j`, stay Gregorian.
    pub fn field(&self, conv: char) -> Option<String> {
        match conv {
            'Y' | 'm' | 'd' | 'e' | 'F' | 'D' => {
                let (year, month, day) = self.converted?;
                Some(match conv {
                    'Y' => year.to_string(),
                    'm' => format!("{month:02}"),
                    'd' => format!("{day:02}"),
                    'e' => format!("{day:2}"),
                    // %F is %+4Y-%m-%d
                    'F' if year > 9999 => format!("+{year}-{month:02}-{day:02}"),
                    'F' => format!("{year:04}-{month:02}-{day:02}"),
                    // %D is %m/%d/%y
                    _ => format!(
                        "{month:02}/{day:02}/{:02}",
                        self.iso_date.year().extended_year().rem_euclid(100)
                    ),
                })
            }
            'B' => format_with!(MONTH_LONG, &self.iso_date),
            // ICU's medium format may include trailing periods (e.g., "febr." for Hungarian),
            // which when combined with locale format strings that also add periods after
            // %b (e.g., "%Y. %b. %d") results in double periods ("febr..").
            // The standard C/POSIX locale via nl_langinfo returns abbreviations
            // WITHOUT trailing periods, so we strip them here for consistency.
            'b' | 'h' => format_with!(MONTH_MEDIUM, &self.iso_date)
                .map(|name| name.trim_end_matches('.').to_string()),
            'A' => format_with!(WEEKDAY_LONG, &self.iso_date),
            'a' => format_with!(WEEKDAY_SHORT, &self.iso_date),
            _ => None,
        }
    }
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
