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
}

cfg_langinfo! {
    /// Cached locale date/time format string
    static DEFAULT_FORMAT_CACHE: OnceLock<&'static str> = OnceLock::new();

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
        unsafe {
            // Set locale from environment variables
            libc::setlocale(libc::LC_TIME, c"".as_ptr());

            // Get the date/time format string
            let d_t_fmt_ptr = libc::nl_langinfo(libc::D_T_FMT);
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

        #[test]
        fn test_locale_detection() {
            // Just verify the function doesn't panic
            let _ = get_locale_default_format();
        }

        #[test]
        fn test_default_format_contains_valid_codes() {
            let format = get_locale_default_format();
            // On platforms without locale support, fallback may not contain all codes
            #[cfg(any(
                target_os = "linux",
                target_vendor = "apple",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd",
                target_os = "dragonfly"
            ))]
            {
                // We only strictly check for timezone because ensure_timezone_in_format guarantees it
                assert!(format.contains("%Z") || format.contains("%z")); // timezone

                // Other components depend on the system locale and might vary (e.g. %F, %c, etc.)
                // so we don't strictly assert %a or %b here.
            }
        }

        #[test]
        fn test_locale_format_structure() {
            let format = get_locale_default_format();
            assert!(!format.is_empty(), "Locale format should not be empty");
            #[cfg(any(
                target_os = "linux",
                target_vendor = "apple",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd",
                target_os = "dragonfly"
            ))]
            {
                let has_date_component = format.contains("%a")
                    || format.contains("%A")
                    || format.contains("%b")
                    || format.contains("%B")
                    || format.contains("%d")
                    || format.contains("%e")
                    || format.contains("%F") // YYYY-MM-DD
                    || format.contains("%D") // MM/DD/YY
                    || format.contains("%x") // Locale's date representation
                    || format.contains("%c") // Locale's date and time
                    || format.contains("%+"); // date(1) extended format
                assert!(has_date_component, "Format should contain date components");

                let has_time_component = format.contains("%H")
                    || format.contains("%I")
                    || format.contains("%k")
                    || format.contains("%l")
                    || format.contains("%r")
                    || format.contains("%R")
                    || format.contains("%T")
                    || format.contains("%X")
                    || format.contains("%c") // Locale's date and time
                    || format.contains("%+"); // date(1) extended format
                assert!(has_time_component, "Format should contain time components");
            }
        }

        #[test]
        fn test_c_locale_format() {
            // Save original locale
            let original_lc_all = std::env::var("LC_ALL").ok();
            let original_lc_time = std::env::var("LC_TIME").ok();
            let original_lang = std::env::var("LANG").ok();

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
                    std::ffi::CStr::from_ptr(d_t_fmt_ptr).to_str().ok()
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

            // Restore original locale
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
