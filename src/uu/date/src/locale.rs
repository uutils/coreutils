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

cfg_langinfo! {
    use core::ffi::CStr;
    use std::sync::OnceLock;

    #[cfg(test)]
    use std::sync::Mutex;

    /// glibc's `_DATE_FMT` has been stable for the last 12 years
    /// being added upstream to libc TODO: update to libc
    const DATE_FMT: libc::nl_item = 0x2006c;
}

cfg_langinfo! {
    /// Cached locale date/time format string
    static DEFAULT_FORMAT_CACHE: OnceLock<&'static [u8]> = OnceLock::new();

    /// Cached locale `D_T_FMT`, the format `%c` stands for.
    static DATETIME_FORMAT_CACHE: OnceLock<Option<&'static str>> = OnceLock::new();

    /// Mutex to serialize setlocale() calls during tests.
    ///
    /// setlocale() is process-global, so parallel tests that call it can
    /// interfere with each other. This mutex ensures only one test accesses
    /// locale functions at a time.
    #[cfg(test)]
    static LOCALE_MUTEX: Mutex<()> = Mutex::new(());

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
        query_nl_langinfo(DATE_FMT)
    }

    /// Returns the locale's combined date and time format (`D_T_FMT`), which
    /// is what `%c` stands for.
    ///
    /// `None` when the locale supplies no format, or when that format is not
    /// valid UTF-8, as happens with legacy charsets such as `zh_TW.euctw`.
    /// Callers then leave `%c` to its POSIX rendering.
    pub fn get_locale_datetime_format() -> Option<&'static str> {
        *DATETIME_FORMAT_CACHE.get_or_init(|| {
            let format = String::from_utf8(query_nl_langinfo(libc::D_T_FMT)?).ok()?;
            Some(&*Box::leak(format.into_boxed_str()))
        })
    }

    /// Reads one `nl_langinfo` item for the locale named by the environment.
    ///
    /// Returns bytes because legacy charsets (e.g. `zh_TW.euctw`) are not UTF-8.
    fn query_nl_langinfo(item: libc::nl_item) -> Option<Vec<u8>> {
        // In tests, acquire mutex to prevent race conditions with setlocale()
        // which is process-global and not thread-safe
        #[cfg(test)]
        let _lock = LOCALE_MUTEX.lock().unwrap();

        unsafe {
            // Set locale from environment variables
            libc::setlocale(libc::LC_TIME, c"".as_ptr());

            let item_ptr = libc::nl_langinfo(item);
            if item_ptr.is_null() {
                return None;
            }

            let value = CStr::from_ptr(item_ptr).to_bytes();
            (!value.is_empty()).then(|| value.to_vec())
        }
    }
}

cfg_langinfo! { else
    /// On platforms without `_DATE_FMT`, fall back to the POSIX format
    pub fn get_locale_default_format() -> &'static [u8] {
        POSIX_DEFAULT_FORMAT
    }

    /// Without a langinfo query, `%c` keeps the POSIX rendering jiff gives it.
    ///
    /// `D_T_FMT` is POSIX, unlike the `_DATE_FMT` extension above, so the
    /// other Unix platforms could answer this too; wiring them up needs a
    /// machine to check them on.
    pub fn get_locale_datetime_format() -> Option<&'static str> {
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
