// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Locale-aware datetime formatting using ICU4X.
//!
//! This module provides thread-safe wrappers around ICU4X's `DateTimeFormatter` to support
//! locale-specific date and time formatting, particularly for `ls --time-style=locale`.
//!
//! # Thread Safety
//!
//! Formatters are cached per-thread using `thread_local!` because ICU4X formatters
//! contain `Rc` types that are not `Send`/`Sync`. This is safe for single-threaded
//! utilities like `ls`.
//!
//! # Locale Support
//!
//! - C/POSIX: Falls back to English month names (backward compatible)
//! - All others: Uses ICU4X with full CLDR locale data (500+ locales)
//!
//! # Examples
//!
//! ```no_run
//! use uucore::i18n::datetime::format_ls_time;
//! use std::time::SystemTime;
//!
//! let timestamp = SystemTime::now();
//! let recent_format = format_ls_time(timestamp, true);  // Shows time
//! let older_format = format_ls_time(timestamp, false); // Shows year
//! ```

use std::cell::RefCell;
use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};

use icu_calendar::{Date, Gregorian};
use icu_datetime::FixedCalendarDateTimeFormatter;
use icu_datetime::fieldsets::{YMD, YMDT};
use icu_locale::Locale;
use icu_time::{DateTime, Time};
use writeable::Writeable;

use crate::i18n::{DEFAULT_LOCALE, UEncoding, get_time_locale};

/// Seconds in a day
const SECS_PER_DAY: i64 = 86_400;
/// Seconds in an hour
const SECS_PER_HOUR: i64 = 3_600;
/// Seconds in a minute
const SECS_PER_MINUTE: i64 = 60;

/// Days in each month for non-leap years
const DAYS_IN_MONTH: [u32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

/// Days in each month for leap years
const DAYS_IN_MONTH_LEAP: [u32; 12] = [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

/// English abbreviated month names (POSIX standard)
const MONTH_ABBR: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// Type alias for date-time formatter (shows both date and time)
type LsDateTimeFormatter = FixedCalendarDateTimeFormatter<Gregorian, YMDT>;

/// Type alias for date-only formatter (shows year, not time)
type LsDateFormatter = FixedCalendarDateTimeFormatter<Gregorian, YMD>;

/// Maximum size of the timestamp format cache (LRU)
const CACHE_SIZE: usize = 16;

/// Cached formatted timestamp entry
#[derive(Clone)]
struct CacheEntry {
    timestamp: SystemTime,
    is_recent: bool,
    formatted: Vec<u8>,
}

/// Thread-local state for formatters and cache
struct FormatterState {
    locale: Option<&'static (Locale, UEncoding)>,
    recent_formatter: Option<LsDateTimeFormatter>,
    older_formatter: Option<LsDateFormatter>,
    cache: VecDeque<CacheEntry>,
}

impl FormatterState {
    const fn new() -> Self {
        Self {
            locale: None,
            recent_formatter: None,
            older_formatter: None,
            cache: VecDeque::new(),
        }
    }

    fn get_cached(&mut self, timestamp: SystemTime, is_recent: bool) -> Option<&[u8]> {
        // Check if we have this exact timestamp cached
        self.cache
            .iter()
            .find(|e| e.timestamp == timestamp && e.is_recent == is_recent)
            .map(|e| e.formatted.as_slice())
    }

    fn cache_result(&mut self, timestamp: SystemTime, is_recent: bool, formatted: Vec<u8>) {
        // Remove oldest entry if cache is full
        if self.cache.len() >= CACHE_SIZE {
            self.cache.pop_back();
        }
        // Add new entry at front (most recently used)
        self.cache.push_front(CacheEntry {
            timestamp,
            is_recent,
            formatted,
        });
    }
}

// Thread-local cache for formatters, locale, and formatted timestamps
thread_local! {
    static FORMATTER_STATE: RefCell<FormatterState> = const { RefCell::new(FormatterState::new()) };
}

/// Initialize a date-time formatter (recent files) with fallback to English locale.
///
/// # Safety
///
/// This function uses `expect()` for the English locale fallback, which should
/// never fail as "en" is a valid BCP-47 locale identifier.
fn init_datetime_formatter(locale: &Locale) -> LsDateTimeFormatter {
    FixedCalendarDateTimeFormatter::try_new(locale.clone().into(), YMDT::short())
        .or_else(|_| {
            // Fallback to en if locale creation fails
            let en_locale: Locale = "en"
                .parse()
                .expect("BUG: 'en' is a valid locale identifier");
            FixedCalendarDateTimeFormatter::try_new(en_locale.into(), YMDT::short())
        })
        .expect("BUG: English (en) locale formatter should never fail")
}

/// Initialize a date-only formatter (older files) with fallback to English locale.
///
/// # Safety
///
/// This function uses `expect()` for the English locale fallback, which should
/// never fail as "en" is a valid BCP-47 locale identifier.
fn init_date_formatter(locale: &Locale) -> LsDateFormatter {
    FixedCalendarDateTimeFormatter::try_new(locale.clone().into(), YMD::short())
        .or_else(|_| {
            // Fallback to en if locale creation fails
            let en_locale: Locale = "en"
                .parse()
                .expect("BUG: 'en' is a valid locale identifier");
            FixedCalendarDateTimeFormatter::try_new(en_locale.into(), YMD::short())
        })
        .expect("BUG: English (en) locale formatter should never fail")
}

/// Format a timestamp for `ls` output, using locale-aware formatting.
///
/// This function automatically selects the appropriate formatter based on the
/// current `LC_TIME` locale setting.
///
/// # Arguments
///
/// * `timestamp` - The timestamp to format
/// * `is_recent` - If `true`, shows time; if `false`, shows year
///
/// # Returns
///
/// A formatted string suitable for `ls -l` output with localized month names.
///
/// # Format Examples
///
/// - C/POSIX locale: "Jan 15 10:50" (recent) or "Jan 15  2024" (older)
/// - en-US locale: "1/15/24, 10:50 AM" (recent) or "1/15/24" (older)
/// - fr-FR locale: "15/01/2024, 10:50" (recent) or "15/01/2024" (older)
/// - de-DE locale: "15.01.24, 10:50" (recent) or "15.01.24" (older)
///
/// # Panics
///
/// This function does not panic. Errors in locale parsing or formatting
/// fall back to English (en) locale formatting.
///
/// # Examples
///
/// ```no_run
/// use std::time::{SystemTime, UNIX_EPOCH, Duration};
/// use uucore::i18n::datetime::format_ls_time;
///
/// let ts = UNIX_EPOCH + Duration::from_secs(1705315800);
/// let recent = format_ls_time(ts, true);
/// let older = format_ls_time(ts, false);
/// ```
pub fn format_ls_time(timestamp: SystemTime, is_recent: bool) -> String {
    // Delegate to write_ls_time to ensure consistency and leverage caching
    let mut buf = Vec::with_capacity(32);
    write_ls_time(&mut buf, timestamp, is_recent);
    // SAFETY: write_ls_time produces valid UTF-8 (either ASCII POSIX format or ICU4X formatted output)
    String::from_utf8(buf).expect("BUG: formatted timestamp should be valid UTF-8")
}

/// Write a timestamp for `ls` output directly into the provided buffer.
/// This avoids intermediate String allocation on the hot path.
pub fn write_ls_time(out: &mut Vec<u8>, timestamp: SystemTime, is_recent: bool) {
    FORMATTER_STATE.with(|state_cell| {
        let mut state = state_cell.borrow_mut();

        // Cache locale lookup on first use
        if state.locale.is_none() {
            state.locale = Some(get_time_locale());
        }
        let (locale, _source) = state.locale.unwrap();

        // Check cache first
        if let Some(cached) = state.get_cached(timestamp, is_recent) {
            out.extend_from_slice(cached);
            return;
        }

        // Format timestamp based on locale
        let formatted = if locale == &DEFAULT_LOCALE {
            // Fast POSIX path: skip ICU conversion entirely
            format_posix_time_direct(timestamp, is_recent).into_bytes()
        } else {
            // Use ICU4X formatters for full locale support
            let dt = system_time_to_icu_datetime(timestamp);
            let mut result = Vec::with_capacity(32);
            struct WriteVec<'a>(&'a mut Vec<u8>);
            impl core::fmt::Write for WriteVec<'_> {
                fn write_str(&mut self, s: &str) -> core::fmt::Result {
                    self.0.extend_from_slice(s.as_bytes());
                    Ok(())
                }
            }

            if is_recent {
                if state.recent_formatter.is_none() {
                    state.recent_formatter = Some(init_datetime_formatter(locale));
                }
                let mut w = WriteVec(&mut result);
                state
                    .recent_formatter
                    .as_ref()
                    .expect("BUG: formatter should be initialized")
                    .format(&dt)
                    .write_to(&mut w)
                    .expect("BUG: write to buffer failed");
            } else {
                if state.older_formatter.is_none() {
                    state.older_formatter = Some(init_date_formatter(locale));
                }
                let mut w = WriteVec(&mut result);
                state
                    .older_formatter
                    .as_ref()
                    .expect("BUG: formatter should be initialized")
                    .format(&dt)
                    .write_to(&mut w)
                    .expect("BUG: write to buffer failed");
            }
            result
        };

        // Cache the result and write to output
        out.extend_from_slice(&formatted);
        state.cache_result(timestamp, is_recent, formatted);
    });
}


/// Convert `SystemTime` to ICU4X `DateTime` for Gregorian calendar.
///
/// This function performs manual epoch-to-calendar conversion because ICU4X
/// does not provide direct `SystemTime` conversion. The implementation accounts
/// for leap years and handles dates from 1970 onwards.
///
/// # Arguments
///
/// * `timestamp` - System time to convert
///
/// # Returns
///
/// ICU4X `DateTime<Gregorian>` representing the given timestamp in UTC.
///
/// # Fallback Behavior
///
/// - Timestamps before Unix epoch (1970-01-01) default to epoch
/// - Invalid date components fall back to 1970-01-01 00:00:00
///
/// # Notes
///
/// This conversion assumes UTC timezone. Locale-specific timezone handling
/// is not currently supported.
fn system_time_to_icu_datetime(timestamp: SystemTime) -> DateTime<Gregorian> {
    // Get duration since UNIX_EPOCH, defaulting to epoch for pre-1970 times
    let duration = timestamp.duration_since(UNIX_EPOCH).unwrap_or_default();

    let secs = duration.as_secs() as i64;

    // Calculate date components
    let days = secs / SECS_PER_DAY;
    let remaining_secs = secs % SECS_PER_DAY;

    // Convert days-since-epoch to year/month/day
    let mut year = 1970_i32;
    let mut days_remaining = days;

    // Iterate through years, accounting for leap years
    // Note: This loop is bounded by reasonable date ranges (1970-9999+)
    while days_remaining >= days_in_year(year) {
        days_remaining -= days_in_year(year);
        year = year.saturating_add(1); // Prevent overflow
    }

    // Convert day-of-year to month and day
    let (month, day) =
        days_to_month_day(days_remaining.saturating_add(1) as u32, is_leap_year(year));

    // Calculate time components
    let hour = ((remaining_secs / SECS_PER_HOUR) % 24) as u8;
    let minute = ((remaining_secs % SECS_PER_HOUR) / SECS_PER_MINUTE) as u8;
    let second = (remaining_secs % SECS_PER_MINUTE) as u8;

    // Create ICU4X Date and DateTime
    let date = Date::try_new_gregorian(year, month, day).unwrap_or_else(|_err| {
        // Fallback to epoch date if conversion fails
        // Note: Invalid date components are extremely rare in normal operation and would indicate
        // a bug in the date calculation logic. In release builds, we silently fall back to epoch.
        #[cfg(debug_assertions)]
        eprintln!(
            "Warning: Invalid date components ({year}-{month:02}-{day:02}): {_err}. Falling back to epoch."
        );
        Date::try_new_gregorian(1970, 1, 1)
            .expect("BUG: Unix epoch date (1970-01-01) should always be valid")
    });

    let time = Time::try_new(hour, minute, second, 0).unwrap_or_else(|_err| {
        // Fallback to midnight if time creation fails
        #[cfg(debug_assertions)]
        eprintln!(
            "Warning: Invalid time components ({hour:02}:{minute:02}:{second:02}): {_err}. Falling back to midnight."
        );
        Time::try_new(0, 0, 0, 0).expect("BUG: Midnight (00:00:00) should always be valid")
    });

    DateTime { date, time }
}

/// Get number of days in a year (accounting for leap years)
#[inline]
const fn days_in_year(year: i32) -> i64 {
    if is_leap_year(year) { 366 } else { 365 }
}

/// Check if a year is a leap year according to Gregorian calendar rules.
///
/// A year is a leap year if:
/// - It is divisible by 4, AND
/// - It is NOT divisible by 100, UNLESS
/// - It is also divisible by 400
///
/// # Examples
///
/// ```
/// # use uucore::i18n::datetime::is_leap_year;
/// assert!(is_leap_year(2000));  // Divisible by 400
/// assert!(is_leap_year(2024));  // Divisible by 4, not by 100
/// assert!(!is_leap_year(1900)); // Divisible by 100 but not 400
/// assert!(!is_leap_year(2023)); // Not divisible by 4
/// ```
#[inline]
pub const fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

/// Convert day-of-year (1-366) to (month, day).
///
/// # Arguments
///
/// * `day_of_year` - Day number in the year (1 = January 1, 365/366 = December 31)
/// * `is_leap` - Whether the year is a leap year
///
/// # Returns
///
/// A tuple of `(month, day)` where month is 1-12 and day is 1-31.
///
/// # Edge Cases
///
/// If `day_of_year` exceeds the days in the year (365 or 366), returns (12, 31).
pub const fn days_to_month_day(day_of_year: u32, is_leap: bool) -> (u8, u8) {
    let days_in_months = if is_leap {
        &DAYS_IN_MONTH_LEAP
    } else {
        &DAYS_IN_MONTH
    };

    let mut remaining = day_of_year;
    let mut i = 0;
    while i < 12 {
        if remaining <= days_in_months[i] {
            return ((i + 1) as u8, remaining as u8);
        }
        remaining -= days_in_months[i];
        i += 1;
    }

    // Fallback for invalid day_of_year (> 365/366)
    // This should not happen in normal operation
    (12, 31)
}

/// Fast POSIX time formatting that skips ICU conversion entirely.
/// This is the hot path for C/POSIX locale and needs to be as fast as possible.
///
/// # Format
///
/// - Recent files: "Mon DD HH:MM" (e.g., "Jan 15 10:50")
/// - Older files: "Mon DD  YYYY" (e.g., "Jan 15  2024")
///
/// Note the two spaces before the year in older file format for alignment.
fn format_posix_time_direct(timestamp: SystemTime, is_recent: bool) -> String {
    // Get duration since UNIX_EPOCH, defaulting to epoch for pre-1970 times
    let duration = timestamp.duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = duration.as_secs() as i64;

    // Fast conversion without creating ICU types
    let (year, month, day, hour, minute) = epoch_to_components_fast(secs);

    if is_recent {
        // Format: "Mon DD HH:MM"
        format!("{} {:2} {:02}:{:02}", month_abbr(month), day, hour, minute)
    } else {
        // Format: "Mon DD  YYYY" (note: two spaces before year)
        format!("{} {:2}  {}", month_abbr(month), day, year)
    }
}

/// Fast epoch-to-components conversion optimized for common date range.
/// Returns (year, month, day, hour, minute) without creating ICU types.
#[inline]
fn epoch_to_components_fast(secs: i64) -> (i32, u8, u8, u8, u8) {
    // Calculate date components
    let days = secs / SECS_PER_DAY;
    let remaining_secs = secs % SECS_PER_DAY;

    // Fast year calculation using average year length
    // 365.2425 days per year in Gregorian calendar ≈ 146097 days per 400 years
    let mut year = 1970_i32;
    let mut days_remaining = days;

    // Fast path for common range (2000-2100)
    if days >= 10957 {  // Days from 1970 to 2000
        year = 2000;
        days_remaining -= 10957;
        
        // Estimate years using average of 365.25 days
        let est_years = (days_remaining / 365) as i32;
        if est_years > 0 {
            year += est_years;
            days_remaining -= days_in_years_fast(est_years);
            
            // Adjust if we overshot
            while days_remaining < 0 {
                year -= 1;
                days_remaining += days_in_year(year);
            }
            while days_remaining >= days_in_year(year) {
                days_remaining -= days_in_year(year);
                year += 1;
            }
        }
    } else {
        // Fallback for dates before 2000
        while days_remaining >= days_in_year(year) {
            days_remaining -= days_in_year(year);
            year += 1;
        }
    }

    // Convert day-of-year to month and day
    let (month, day) = days_to_month_day(days_remaining.saturating_add(1) as u32, is_leap_year(year));

    // Calculate time components
    let hour = ((remaining_secs / SECS_PER_HOUR) % 24) as u8;
    let minute = ((remaining_secs % SECS_PER_HOUR) / SECS_PER_MINUTE) as u8;

    (year, month, day, hour, minute)
}

/// Calculate total days in a span of years (approximate for fast path)
#[inline]
const fn days_in_years_fast(years: i32) -> i64 {
    // Use average of 365.25 days per year
    (years as i64) * 365 + (years as i64) / 4
}

/// Get English abbreviated month name for POSIX/C locale.
///
/// # Arguments
///
/// * `month` - Month number (1-12, where 1 = January)
///
/// # Returns
///
/// Three-letter English month abbreviation, or "ERR" for invalid months.
///
/// # Examples
///
/// ```
/// # use uucore::i18n::datetime::month_abbr;
/// assert_eq!(month_abbr(1), "Jan");
/// assert_eq!(month_abbr(12), "Dec");
/// assert_eq!(month_abbr(13), "ERR"); // Invalid
/// ```
#[inline]
pub const fn month_abbr(month: u8) -> &'static str {
    if month >= 1 && month <= 12 {
        MONTH_ABBR[(month - 1) as usize]
    } else {
        "ERR" // More descriptive than "???"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_posix_format_recent() {
        // 2024-01-15 10:50:00 UTC
        let ts = UNIX_EPOCH + Duration::from_secs(1705315800);
        let formatted = format_posix_time_direct(ts, true);
        assert!(formatted.contains("Jan"));
        assert!(formatted.contains("15"));
        assert!(formatted.contains("10:50"));
    }

    #[test]
    fn test_posix_format_older() {
        // 2024-01-15 10:30:00 UTC
        let ts = UNIX_EPOCH + Duration::from_secs(1705315800);
        let formatted = format_posix_time_direct(ts, false);
        assert!(formatted.contains("Jan"));
        assert!(formatted.contains("15"));
        assert!(formatted.contains("2024"));
        // Check for two spaces before year
        assert!(formatted.contains("  2024"));
    }

    #[test]
    fn test_month_abbreviations() {
        assert_eq!(month_abbr(1), "Jan");
        assert_eq!(month_abbr(6), "Jun");
        assert_eq!(month_abbr(12), "Dec");
        assert_eq!(month_abbr(0), "ERR", "Should handle invalid month 0");
        assert_eq!(month_abbr(13), "ERR", "Should handle invalid month 13");
    }

    #[test]
    fn test_leap_year() {
        // Divisible by 400
        assert!(is_leap_year(2000));
        assert!(is_leap_year(2400));

        // Divisible by 4 but not 100
        assert!(is_leap_year(2024));
        assert!(is_leap_year(2020));

        // Divisible by 100 but not 400
        assert!(!is_leap_year(1900));
        assert!(!is_leap_year(2100));

        // Not divisible by 4
        assert!(!is_leap_year(2023));
        assert!(!is_leap_year(2021));
    }

    #[test]
    fn test_system_time_conversion_accuracy() {
        // Test: 2024-01-15 10:50:00 UTC
        let ts = UNIX_EPOCH + Duration::from_secs(1705315800);
        let dt = system_time_to_icu_datetime(ts);

        // Verify date components
        assert_eq!(dt.date.era_year().year, 2024, "Year should be 2024");
        assert_eq!(dt.date.month().ordinal, 1, "Month should be January (1)");
        assert_eq!(dt.date.day_of_month().0, 15, "Day should be 15");

        // Verify time components
        assert_eq!(dt.time.hour.number(), 10, "Hour should be 10");
        assert_eq!(dt.time.minute.number(), 50, "Minute should be 50");
        assert_eq!(dt.time.second.number(), 0, "Second should be 0");
    }

    #[test]
    fn test_epoch_timestamp() {
        // Unix epoch: 1970-01-01 00:00:00
        let dt = system_time_to_icu_datetime(UNIX_EPOCH);
        assert_eq!(dt.date.era_year().year, 1970);
        assert_eq!(dt.date.month().ordinal, 1);
        assert_eq!(dt.date.day_of_month().0, 1);
        assert_eq!(dt.time.hour.number(), 0);
        assert_eq!(dt.time.minute.number(), 0);
    }

    #[test]
    fn test_leap_year_date() {
        // Feb 29, 2024 (leap year)
        let ts = UNIX_EPOCH + Duration::from_secs(1709164800); // 2024-02-29 00:00:00
        let dt = system_time_to_icu_datetime(ts);
        assert_eq!(dt.date.era_year().year, 2024);
        assert_eq!(dt.date.month().ordinal, 2); // February
        assert_eq!(dt.date.day_of_month().0, 29);
    }

    #[test]
    fn test_days_to_month_day() {
        // Test various day-of-year values
        assert_eq!(days_to_month_day(1, false), (1, 1)); // Jan 1
        assert_eq!(days_to_month_day(32, false), (2, 1)); // Feb 1
        assert_eq!(days_to_month_day(60, false), (3, 1)); // Mar 1 (non-leap)
        assert_eq!(days_to_month_day(60, true), (2, 29)); // Feb 29 (leap)
        assert_eq!(days_to_month_day(365, false), (12, 31)); // Dec 31 (non-leap)
        assert_eq!(days_to_month_day(366, true), (12, 31)); // Dec 31 (leap)
    }
}
