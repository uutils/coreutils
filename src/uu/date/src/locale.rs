// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Locale detection for time format preferences

/// Format used when the locale does not provide a date/time format.
const POSIX_DEFAULT_FORMAT: &[u8] = b"%a %b %e %X %Z %Y";

// `_DATE_FMT` is the only langinfo item that spells the full date line the way
// `date` prints it, timezone included. It is a glibc extension, so everywhere
// else (Android, the BSDs, macOS, Redox, non-unix) we use the POSIX format:
// `D_T_FMT` would be locale-aware but has no timezone, which `date` must print.

// Macro to reduce cfg duplication across the module; `cfg_langinfo!(else ...)`
// gates the items for the platforms that do not have `_DATE_FMT`.
macro_rules! cfg_langinfo {
    (else $($item:item)*) => {
        $(
            #[cfg(not(all(target_os = "linux", not(target_env = "musl"))))]
            $item
        )*
    };
    ($($item:item)*) => {
        $(
            #[cfg(all(target_os = "linux", not(target_env = "musl")))]
            $item
        )*
    };
}

// The POSIX `nl_langinfo` items used by `%x`, `%X` and `%r` (`D_FMT`, `T_FMT`,
// `T_FMT_AMPM`) are standard, so they are available on far more targets than
// glibc's `_DATE_FMT` extension: everywhere `nl_langinfo` itself exists, which
// is every unix except Android, Cygwin and Redox. `cfg_nl_langinfo!(else ...)`
// gates the fallbacks for the remaining targets and is the exact inverse of the
// plain form, so no target can compile both or neither.
macro_rules! cfg_nl_langinfo {
    (else $($item:item)*) => {
        $(
            #[cfg(not(all(
                unix,
                not(target_os = "android"),
                not(target_os = "cygwin"),
                not(target_os = "redox")
            )))]
            $item
        )*
    };
    ($($item:item)*) => {
        $(
            #[cfg(all(
                unix,
                not(target_os = "android"),
                not(target_os = "cygwin"),
                not(target_os = "redox")
            ))]
            $item
        )*
    };
}

cfg_nl_langinfo! {
    use core::ffi::CStr;

    #[cfg(test)]
    use std::sync::Mutex;

    /// `D_FMT` — locale date format (used by `%x`)
    const D_FMT_ITEM: libc::nl_item = libc::D_FMT;
    /// `T_FMT` — locale time format (used by `%X`)
    const T_FMT_ITEM: libc::nl_item = libc::T_FMT;
    /// `T_FMT_AMPM` — locale 12-hour time format (used by `%r`)
    const T_FMT_AMPM_ITEM: libc::nl_item = libc::T_FMT_AMPM;
    /// Locale AM marker (used by `%p`/`%r`).
    const AM_STR_ITEM: libc::nl_item = libc::AM_STR;
    /// Locale PM marker (used by `%p`/`%r`).
    const PM_STR_ITEM: libc::nl_item = libc::PM_STR;

    /// Mutex to serialize setlocale() calls during tests.
    ///
    /// setlocale() is process-global, so parallel tests that call it can
    /// interfere with each other. This mutex ensures only one test accesses
    /// locale functions at a time.
    #[cfg(test)]
    static LOCALE_MUTEX: Mutex<()> = Mutex::new(());
}

cfg_langinfo! {
    use std::sync::OnceLock;

    /// glibc's `_DATE_FMT` has been stable for the last 12 years
    /// being added upstream to libc TODO: update to libc
    const DATE_FMT: libc::nl_item = 0x2006c;
}

cfg_langinfo! {
    /// Cached locale date/time format string
    static DEFAULT_FORMAT_CACHE: OnceLock<&'static [u8]> = OnceLock::new();

    /// Returns the default date format string for the current locale.
    ///
    /// This is the locale's `date_fmt`/`d_t_fmt` used verbatim, so the output
    /// matches what `date +"$(locale date_fmt)"` produces. It is returned as
    /// bytes because legacy charsets (e.g. `zh_TW.euctw`) are not UTF-8.
    pub fn get_locale_default_format() -> &'static [u8] {
        DEFAULT_FORMAT_CACHE.get_or_init(|| {
            // Try to get locale format string
            if let Some(format) = get_locale_format_string() {
                return Box::leak(format.into_boxed_slice());
            }

            // Fallback: use the POSIX locale format
            POSIX_DEFAULT_FORMAT
        })
    }

    /// Retrieves the date/time format string from the system locale
    fn get_locale_format_string() -> Option<Vec<u8>> {
        // In tests, acquire mutex to prevent race conditions with setlocale()
        // which is process-global and not thread-safe
        #[cfg(test)]
        let _lock = LOCALE_MUTEX.lock().unwrap();

        unsafe {
            // Set locale from environment variables
            libc::setlocale(libc::LC_TIME, c"".as_ptr());

            // Get the date/time format string
            let d_t_fmt_ptr = libc::nl_langinfo(DATE_FMT);
            if d_t_fmt_ptr.is_null() {
                return None;
            }

            let format = CStr::from_ptr(d_t_fmt_ptr).to_bytes();
            (!format.is_empty()).then(|| format.to_vec())
        }
    }
}

cfg_langinfo! { else
    /// On platforms without `_DATE_FMT`, fall back to the POSIX format
    pub fn get_locale_default_format() -> &'static [u8] {
        POSIX_DEFAULT_FORMAT
    }
}

cfg_nl_langinfo! {
    /// Applies the environment's `LC_TIME` locale, reporting whether it exists.
    fn set_locale_time_from_environment() -> bool {
        unsafe { !libc::setlocale(libc::LC_TIME, c"".as_ptr()).is_null() }
    }

    /// Reads a `nl_langinfo` item for the environment's `LC_TIME` locale,
    /// treating an empty value as unavailable.
    fn query_nl_langinfo(item: libc::nl_item) -> Option<String> {
        query_nl_langinfo_inner(item, false)
    }

    /// Reads a `nl_langinfo` item, keeping an explicitly empty value.
    fn query_nl_langinfo_allow_empty(item: libc::nl_item) -> Option<String> {
        query_nl_langinfo_inner(item, true)
    }

    fn query_nl_langinfo_inner(item: libc::nl_item, allow_empty: bool) -> Option<String> {
        // In tests, acquire mutex to prevent race conditions with setlocale()
        // which is process-global and not thread-safe
        #[cfg(test)]
        let _lock = LOCALE_MUTEX.lock().unwrap();

        if !set_locale_time_from_environment() {
            return None;
        }

        unsafe {
            let ptr = libc::nl_langinfo(item);
            if ptr.is_null() {
                return None;
            }

            let s = CStr::from_ptr(ptr).to_str().ok()?;
            if s.is_empty() && !allow_empty {
                return None;
            }

            Some(s.to_string())
        }
    }

    /// Returns the locale date format (`D_FMT`) used by `%x`.
    pub fn get_locale_date_format() -> Option<String> {
        query_nl_langinfo(D_FMT_ITEM)
    }

    /// Returns the locale time format (`T_FMT`) used by `%X`.
    pub fn get_locale_time_format() -> Option<String> {
        query_nl_langinfo(T_FMT_ITEM)
    }

    /// Resolve a locale's `T_FMT_AMPM`, distinguishing an explicitly empty
    /// value from an unavailable value.
    fn ampm_format_or_default(format: Option<String>) -> String {
        match format.as_deref() {
            Some("") => "%H:%M:%S".to_string(),
            None => "%I:%M:%S %p".to_string(),
            Some(value) => value.to_string(),
        }
    }

    /// Returns the locale 12-hour time format (`T_FMT_AMPM`) used by `%r`.
    /// GNU date falls back to `%I:%M:%S %p` if it is unavailable.
    /// However, if a locale explicitly defines it as empty (like French),
    /// it uses `%H:%M:%S`.
    pub fn get_locale_time_ampm_format() -> String {
        ampm_format_or_default(query_nl_langinfo_allow_empty(T_FMT_AMPM_ITEM))
    }

    /// Returns the locale's AM and PM markers used by `%p` and `%P`.
    pub fn get_locale_ampm_markers() -> Option<(String, String)> {
        Some((
            query_nl_langinfo_allow_empty(AM_STR_ITEM)?,
            query_nl_langinfo_allow_empty(PM_STR_ITEM)?,
        ))
    }
}

cfg_nl_langinfo! { else
    /// Fallback for platforms without `nl_langinfo`.
    pub fn get_locale_date_format() -> Option<String> {
        None
    }

    /// Fallback for platforms without `nl_langinfo`.
    pub fn get_locale_time_format() -> Option<String> {
        None
    }

    /// Fallback for platforms without `nl_langinfo`.
    pub fn get_locale_time_ampm_format() -> String {
        "%I:%M:%S %p".to_string()
    }

    /// Fallback for platforms without `nl_langinfo`.
    pub fn get_locale_ampm_markers() -> Option<(String, String)> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::POSIX_DEFAULT_FORMAT;

    /// `date` with no format has to print the timezone, so the fallback used
    /// wherever the locale offers no date format must carry %Z.
    #[test]
    fn test_posix_default_format_has_timezone() {
        assert!(POSIX_DEFAULT_FORMAT.windows(2).any(|pair| pair == b"%Z"));
    }

    cfg_langinfo! { else
        /// Platforms without glibc's `_DATE_FMT` get the POSIX format, not
        /// `D_T_FMT`, which would drop the timezone.
        #[test]
        fn test_default_format_without_date_fmt() {
            assert_eq!(super::get_locale_default_format(), POSIX_DEFAULT_FORMAT);
        }
    }

    cfg_nl_langinfo! {
        use super::{LOCALE_MUTEX, ampm_format_or_default, set_locale_time_from_environment};
        use core::ffi::CStr;

        #[test]
        fn test_ampm_format_distinguishes_empty_and_unavailable() {
            assert_eq!(ampm_format_or_default(Some(String::new())), "%H:%M:%S");
            assert_eq!(ampm_format_or_default(None), "%I:%M:%S %p");
            assert_eq!(
                ampm_format_or_default(Some("%I:%M:%S %p".to_string())),
                "%I:%M:%S %p"
            );
        }

        #[test]
        fn test_setlocale_failure_is_reported() {
            let _lock = LOCALE_MUTEX.lock().unwrap();
            let original_lc_all = std::env::var_os("LC_ALL");
            let original_lc_time = std::env::var_os("LC_TIME");
            let original_lang = std::env::var_os("LANG");
            let original_process_locale = unsafe {
                let ptr = libc::setlocale(libc::LC_TIME, std::ptr::null());
                if ptr.is_null() {
                    None
                } else {
                    CStr::from_ptr(ptr).to_str().ok().map(ToString::to_string)
                }
            };

            unsafe {
                std::env::set_var("LC_ALL", "__hermes_locale_that_does_not_exist__");
                std::env::remove_var("LC_TIME");
                std::env::remove_var("LANG");
            }
            let result = set_locale_time_from_environment();

            unsafe {
                if let Some(value) = original_lc_all {
                    std::env::set_var("LC_ALL", value);
                } else {
                    std::env::remove_var("LC_ALL");
                }
                if let Some(value) = original_lc_time {
                    std::env::set_var("LC_TIME", value);
                } else {
                    std::env::remove_var("LC_TIME");
                }
                if let Some(value) = original_lang {
                    std::env::set_var("LANG", value);
                } else {
                    std::env::remove_var("LANG");
                }
                if let Some(locale) = original_process_locale {
                    let c_locale = std::ffi::CString::new(locale).unwrap();
                    libc::setlocale(libc::LC_TIME, c_locale.as_ptr());
                } else {
                    libc::setlocale(libc::LC_TIME, c"".as_ptr());
                }
            }

            assert!(!result, "invalid environment locale must be reported");
        }
    }

    cfg_langinfo! {
        use super::*;

        /// Expands a format string with a fixed test date (Monday, January 15,
        /// 2024, 14:30:45 UTC), so format strings can be validated by their
        /// output rather than by looking for literal format codes.
        ///
        /// Returns `None` when the format cannot be expanded: a legacy-charset
        /// locale (e.g. `zh_CN.GB18030`) hands us bytes that are not UTF-8, and
        /// callers must skip rather than assert on an empty expansion.
        fn expand_format_with_test_date(format: &[u8]) -> Option<String> {
            use jiff::civil::date;
            use jiff::fmt::strtime;

            let format = std::str::from_utf8(format).ok()?;

            // Create test timestamp: Monday, January 15, 2024, 14:30:45 UTC
            let test_date = date(2024, 1, 15).at(14, 30, 45, 0).in_tz("UTC").ok()?;

            // Expand the format string with the test date
            strtime::format(format, &test_date).ok()
        }

        #[test]
        fn test_locale_detection() {
            // Just verify the function doesn't panic
            let _ = get_locale_default_format();
        }

        #[test]
        fn test_default_format_contains_valid_codes() {
            let format = get_locale_default_format();

            let Some(expanded) = expand_format_with_test_date(format) else {
                return;
            };

            // Verify expanded output contains expected components
            // Test date: Monday, January 15, 2024, 14:30:45
            assert!(
                expanded.contains("Mon") || expanded.contains("Monday"),
                "Expanded format should contain weekday name, got: {expanded}"
            );

            assert!(
                expanded.contains("Jan") || expanded.contains("January"),
                "Expanded format should contain month name, got: {expanded}"
            );

            assert!(
                expanded.contains("2024") || expanded.contains("24"),
                "Expanded format should contain year, got: {expanded}"
            );
        }

        #[test]
        fn test_locale_format_structure() {
            // Verify we're using actual locale format strings, not hardcoded ones
            let format = get_locale_default_format();

            // The format should not be empty
            assert!(!format.is_empty(), "Locale format should not be empty");

            let Some(expanded) = expand_format_with_test_date(format) else {
                return;
            };

            // Verify expanded output contains date components
            // Test date: Monday, January 15, 2024
            let has_date_component = expanded.contains("15")     // day
                || expanded.contains("Jan")                      // month name
                || expanded.contains("January")                  // full month
                || expanded.contains("Mon")                      // weekday
                || expanded.contains("Monday");                  // full weekday

            assert!(
                has_date_component,
                "Expanded format should contain date components, got: {expanded}"
            );

            // Verify expanded output contains time components
            // Test time: 14:30:45
            let has_time_component = expanded.contains("14")     // 24-hour
                || expanded.contains("02")                       // 12-hour
                || expanded.contains("30")                       // minutes
                || expanded.contains(':')                        // time separator
                || expanded.contains("PM")                       // AM/PM indicator
                || expanded.contains("pm");

            assert!(
                has_time_component,
                "Expanded format should contain time components, got: {expanded}"
            );
        }

        #[test]
        fn test_c_locale_format() {
            // Acquire mutex to prevent interference with other tests
            let _lock = LOCALE_MUTEX.lock().unwrap();

            // Save original locale (both environment and process locale)
            let original_lc_all = std::env::var_os("LC_ALL");
            let original_lc_time = std::env::var_os("LC_TIME");
            let original_lang = std::env::var_os("LANG");

            // Save current process locale
            let original_process_locale = unsafe {
                let ptr = libc::setlocale(libc::LC_TIME, std::ptr::null());
                if ptr.is_null() {
                    None
                } else {
                    CStr::from_ptr(ptr).to_str().ok().map(ToString::to_string)
                }
            };

            unsafe {
                // Set C locale
                std::env::set_var("LC_ALL", "C");
                std::env::remove_var("LC_TIME");
                std::env::remove_var("LANG");
            }

            // Get the locale format
            let format = unsafe {
                libc::setlocale(libc::LC_TIME, c"C".as_ptr());
                let d_t_fmt_ptr = libc::nl_langinfo(libc::D_T_FMT);
                if d_t_fmt_ptr.is_null() {
                    None
                } else {
                    CStr::from_ptr(d_t_fmt_ptr).to_str().ok()
                }
            };

            if let Some(locale_format) = format {
                // C locale typically uses 24-hour format
                // Common patterns: %H (24-hour with leading zero) or %T (HH:MM:SS)
                let uses_24_hour = locale_format.contains("%H")
                    || locale_format.contains("%T")
                    || locale_format.contains("%R");
                assert!(uses_24_hour, "C locale should use 24-hour format, got: {locale_format}");
            }

            // Restore original environment variables
            unsafe {
                if let Some(val) = original_lc_all {
                    std::env::set_var("LC_ALL", val);
                } else {
                    std::env::remove_var("LC_ALL");
                }
                if let Some(val) = original_lc_time {
                    std::env::set_var("LC_TIME", val);
                } else {
                    std::env::remove_var("LC_TIME");
                }
                if let Some(val) = original_lang {
                    std::env::set_var("LANG", val);
                } else {
                    std::env::remove_var("LANG");
                }
            }

            // Restore original process locale
            unsafe {
                if let Some(locale) = original_process_locale {
                    let c_locale = std::ffi::CString::new(locale).unwrap();
                    libc::setlocale(libc::LC_TIME, c_locale.as_ptr());
                } else {
                    // Restore from environment
                    libc::setlocale(libc::LC_TIME, c"".as_ptr());
                }
            }
        }

        #[test]
        fn test_format_is_locale_verbatim() {
            // The locale format must be used as-is: no timezone specifier is
            // injected, otherwise `date` and `date +"$(locale date_fmt)"`
            // would disagree for locales whose format has no %Z.
            let format = get_locale_default_format();
            let Some(locale_format) = get_locale_format_string() else {
                assert_eq!(format, POSIX_DEFAULT_FORMAT);
                return;
            };
            assert_eq!(format, locale_format);
        }
    }
}
