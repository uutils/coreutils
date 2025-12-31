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

    /// Replaces %c, %x, %X with their locale-specific format strings.
    ///
    /// If a flag like `^` is present (e.g., `%^c`), it is distributed to the
    /// sub-specifiers within the locale string.
    pub fn expand_locale_format(format: &str) -> std::borrow::Cow<'_, str> {
        let mut result = String::with_capacity(format.len());
        let mut chars = format.chars().peekable();
        let mut modified = false;

        while let Some(c) = chars.next() {
            if c != '%' {
                result.push(c);
                continue;
            }

            // Capture flags
            let mut flags = Vec::new();
            while let Some(&peek) = chars.peek() {
                 match peek {
                    '_' | '-' | '0' | '^' | '#' => {
                        flags.push(peek);
                        chars.next();
                    },
                    _ => break,
                 }
            }

            match chars.peek() {
                Some(&spec @ ('c' | 'x' | 'X')) => {
                    chars.next();

                    let item = match spec {
                        'c' => libc::D_T_FMT,
                        'x' => libc::D_FMT,
                        'X' => libc::T_FMT,
                        _ => unreachable!(),
                    };

                    if let Some(s) = get_langinfo(item) {
                        // If the user requested uppercase (%^c), distribute that flag
                        // to the expanded specifiers.
                        let replacement = if flags.contains(&'^') {
                            distribute_flag(&s, '^')
                        } else {
                            s
                        };
                        result.push_str(&replacement);
                        modified = true;
                    } else {
                        // Reconstruct original sequence if lookup fails
                        result.push('%');
                        result.extend(flags);
                        result.push(spec);
                    }
                },
                Some(_) | None => {
                    // Not a locale specifier, or end of string.
                    // Push captured flags and let loop handle the next char.
                    result.push('%');
                    result.extend(flags);
                }
            }
        }

        if modified {
            std::borrow::Cow::Owned(result)
        } else {
            std::borrow::Cow::Borrowed(format)
        }
    }

    /// Distributes the given flag to all format specifiers in the string.
    ///
    /// For example, if the format string is "%a %b" and the flag is '^',
    /// the result will be "%^a %^b". Literal percent signs ("%%") are skipped
    /// and left as is.
    fn distribute_flag(fmt: &str, flag: char) -> String {
        let mut res = String::with_capacity(fmt.len() * 2);
        let mut chars = fmt.chars().peekable();
        while let Some(c) = chars.next() {
            res.push(c);
            if c == '%' {
                if let Some(&n) = chars.peek() {
                    if n == '%' {
                         chars.next();
                         res.push('%');
                    } else {
                        res.push(flag);
                    }
                }
            }
        }
        res
    }

    /// Retrieves the date/time format string from the system locale (D_T_FMT, D_FMT, T_FMT)
    pub fn get_langinfo(item: libc::nl_item) -> Option<String> {
        // In tests, acquire mutex to prevent race conditions with setlocale()
        // which is process-global and not thread-safe
        #[cfg(test)]
        let _lock = LOCALE_MUTEX.lock().unwrap();

        unsafe {
            // Set locale from environment variables
            libc::setlocale(libc::LC_TIME, c"".as_ptr());

            // Get the date/time format string
            let fmt_ptr = libc::nl_langinfo(item);
            if fmt_ptr.is_null() {
                return None;
            }

            let format = CStr::from_ptr(fmt_ptr).to_str().ok()?;
            if format.is_empty() {
                return None;
            }

            Some(format.to_string())
        }
    }

    /// Retrieves the date/time format string from the system locale
    fn get_locale_format_string() -> Option<String> {
        get_langinfo(libc::D_T_FMT)
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

#[cfg(not(any(
    target_os = "linux",
    target_vendor = "apple",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
)))]
pub fn expand_locale_format(format: &str) -> std::borrow::Cow<'_, str> {
    std::borrow::Cow::Borrowed(format)
}

#[cfg(test)]
mod tests {
    cfg_langinfo! {
        use super::*;

        #[test]
        fn test_locale_detection() {
            // Just verify the function doesn't panic
            let _ = get_locale_default_format();
        }

        #[test]
        fn test_default_format_contains_valid_codes() {
            let format = get_locale_default_format();
            assert!(format.contains("%a")); // abbreviated weekday
            assert!(format.contains("%b")); // abbreviated month
            assert!(format.contains("%Y") || format.contains("%y")); // year (4-digit or 2-digit)
            assert!(format.contains("%Z")); // timezone
        }

        #[test]
        fn test_locale_format_structure() {
            // Verify we're using actual locale format strings, not hardcoded ones
            let format = get_locale_default_format();

            // The format should not be empty
            assert!(!format.is_empty(), "Locale format should not be empty");

            // Should contain date/time components
            let has_date_component = format.contains("%a")
                || format.contains("%A")
                || format.contains("%b")
                || format.contains("%B")
                || format.contains("%d")
                || format.contains("%e");
            assert!(has_date_component, "Format should contain date components");

            // Should contain time component (hour)
            let has_time_component = format.contains("%H")
                || format.contains("%I")
                || format.contains("%k")
                || format.contains("%l")
                || format.contains("%r")
                || format.contains("%R")
                || format.contains("%T")
                || format.contains("%X");
            assert!(has_time_component, "Format should contain time components");
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
                    CStr::from_ptr(ptr).to_str().ok().map(|s| s.to_string())
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

    #[test]
    #[cfg(any(
        target_os = "linux",
        target_vendor = "apple",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd",
        target_os = "dragonfly"
    ))]
    fn test_distribute_flag() {
        // Standard distribution
        assert_eq!(distribute_flag("%a %b", '^'), "%^a %^b");
        // Ignore literals
        assert_eq!(distribute_flag("foo %a bar", '_'), "foo %_a bar");
        // Skip escaped percent signs
        assert_eq!(distribute_flag("%% %a", '^'), "%% %^a");
        // Handle flags that might already exist
        assert_eq!(distribute_flag("%_a", '^'), "%^_a");
    }

    #[test]
    #[cfg(any(
        target_os = "linux",
        target_vendor = "apple",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd",
        target_os = "dragonfly"
    ))]
    fn test_expand_locale_format_basic() {
        let format = "%^x";
        let expanded = expand_locale_format(format).to_string().to_lowercase();

        assert_ne!(expanded, "%^x", "Should have expanded %^x");
        assert!(expanded.contains("%^d"), "Should contain %^d");
        assert!(expanded.contains("%^m"), "Should contain %^m");
        assert!(expanded.contains("%^y"), "Should contain %^y");
    }
}
