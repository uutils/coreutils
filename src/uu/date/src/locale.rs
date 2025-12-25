// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Locale detection for time format preferences

// nl_langinfo is available on glibc (Linux), Apple platforms, and BSDs
// but not on Android, Redox or other minimal Unix systems

// Macro to reduce cfg duplication across the module
macro_rules! cfg_langinfo {
    ($($item:item)*) => {
        $(
            #[cfg(any(
                target_os = "linux",
                target_vendor = "apple",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd",
                target_os = "dragonfly"
            ))]
            $item
        )*
    }
}

cfg_langinfo! {
    use std::ffi::CStr;
    use std::sync::OnceLock;
    use nix::libc;

    #[cfg(test)]
    use std::sync::Mutex;

    /// glibc's `_DATE_FMT` has been stable for the last 12 years
    /// being added upstream to libc TODO: update to libc
    #[cfg(target_os = "linux")]
    const DATE_FMT: libc::nl_item = 0x2006c;
    #[cfg(not(target_os = "linux"))]
    const DATE_FMT: libc::nl_item = libc::D_T_FMT;
}

cfg_langinfo! {
    /// Cached locale date/time format string
    static DEFAULT_FORMAT_CACHE: OnceLock<&'static str> = OnceLock::new();

    /// Mutex to serialize setlocale() calls during tests.
    ///
    /// setlocale() is process-global, so parallel tests that call it can
    /// interfere with each other. This mutex ensures only one test accesses
    /// locale functions at a time.
    #[cfg(test)]
    static LOCALE_MUTEX: Mutex<()> = Mutex::new(());

    /// Returns the default date format string for the current locale.
    ///
    /// The format respects locale preferences for time display (12-hour vs 24-hour),
    /// component ordering, and numeric formatting conventions. Ensures timezone
    /// information is included in the output.
    pub fn get_locale_default_format() -> &'static str {
        DEFAULT_FORMAT_CACHE.get_or_init(|| {
            // Try to get locale format string
            if let Some(format) = get_locale_format_string() {
                let format_with_tz = ensure_timezone_in_format(&format);
                return Box::leak(format_with_tz.into_boxed_str());
            }

            // Fallback: use 24-hour format as safe default
            "%a %b %e %X %Z %Y"
        })
    }

    /// Retrieves the date/time format string from the system locale
    fn get_locale_format_string() -> Option<String> {
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

            let format = CStr::from_ptr(d_t_fmt_ptr).to_str().ok()?;
            if format.is_empty() {
                return None;
            }

            Some(format.to_string())
        }
    }

    /// Ensures the format string includes timezone (%Z)
    fn ensure_timezone_in_format(format: &str) -> String {
        if format.contains("%Z") {
            return format.to_string();
        }

        // Try to insert %Z before year specifier (%Y or %y)
        if let Some(pos) = format.find("%Y").or_else(|| format.find("%y")) {
            let mut result = String::with_capacity(format.len() + 3);
            result.push_str(&format[..pos]);
            result.push_str("%Z ");
            result.push_str(&format[pos..]);
            result
        } else {
            // No year found, append %Z at the end
            format.to_string() + " %Z"
        }
    }
}

/// On platforms without nl_langinfo support, use 24-hour format by default
#[cfg(not(any(
    target_os = "linux",
    target_vendor = "apple",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
)))]
pub fn get_locale_default_format() -> &'static str {
    "%a %b %e %X %Z %Y"
}

#[cfg(test)]
mod tests {
    cfg_langinfo! {
        use super::*;

        /// Helper function to expand a format string with a known test date
        ///
        /// Uses a fixed test date: Monday, January 15, 2024, 14:30:45 UTC
        /// This allows us to validate format strings by checking their expanded output
        /// rather than looking for literal format codes.
        fn expand_format_with_test_date(format: &str) -> String {
            use jiff::civil::date;
            use jiff::fmt::strtime;

            // Create test timestamp: Monday, January 15, 2024, 14:30:45 UTC
            let Ok(test_date) = date(2024, 1, 15).at(14, 30, 45, 0).in_tz("UTC") else {
                return String::new();
            };

            // Expand the format string with the test date
            strtime::format(format, &test_date).unwrap_or_default()
        }

        #[test]
        fn test_locale_detection() {
            // Just verify the function doesn't panic
            let _ = get_locale_default_format();
        }

        #[test]
        fn test_default_format_contains_valid_codes() {
            let format = get_locale_default_format();

            let expanded = expand_format_with_test_date(format);

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

            // Keep literal %Z check - this is enforced by ensure_timezone_in_format()
            assert!(
                format.contains("%Z"),
                "Format string must contain %Z timezone (enforced by ensure_timezone_in_format)"
            );
        }

        #[test]
        fn test_locale_format_structure() {
            // Verify we're using actual locale format strings, not hardcoded ones
            let format = get_locale_default_format();

            // The format should not be empty
            assert!(!format.is_empty(), "Locale format should not be empty");

            let expanded = expand_format_with_test_date(format);

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
            let original_lc_all = std::env::var("LC_ALL").ok();
            let original_lc_time = std::env::var("LC_TIME").ok();
            let original_lang = std::env::var("LANG").ok();

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
        fn test_timezone_included_in_format() {
            // The implementation should ensure %Z is present
            let format = get_locale_default_format();
            assert!(
                format.contains("%Z") || format.contains("%z"),
                "Format should contain timezone indicator: {format}"
            );
        }
    }
}
