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
                #[cfg(test)]
                eprintln!("DEBUG: get_locale_default_format: Using system format: '{}'", format);
                let format_with_tz = ensure_timezone_in_format(&format);
                #[cfg(test)]
                eprintln!("DEBUG: get_locale_default_format: After timezone adjustment: '{}'", format_with_tz);
                return Box::leak(format_with_tz.into_boxed_str());
            }

            #[cfg(test)]
            eprintln!("DEBUG: get_locale_default_format: No system format, using fallback");
            // Fallback: use 24-hour format as safe default
            "%a %b %e %X %Z %Y"
        })
    }

    /// Retrieves the date/time format string from the system locale
    fn get_locale_format_string() -> Option<String> {
        unsafe {
            // Set locale from environment variables
            let _locale_result = libc::setlocale(libc::LC_TIME, c"".as_ptr());
            #[cfg(test)]
            {
                let current_locale = if _locale_result.is_null() {
                    "NULL".to_string()
                } else {
                    CStr::from_ptr(_locale_result).to_string_lossy().into_owned()
                };
                eprintln!("DEBUG: get_locale_format_string: setlocale result: '{}'", current_locale);
            }

            // Get the date/time format string
            let d_t_fmt_ptr = libc::nl_langinfo(libc::D_T_FMT);
            if d_t_fmt_ptr.is_null() {
                #[cfg(test)]
                eprintln!("DEBUG: get_locale_format_string: nl_langinfo returned null pointer");
                return None;
            }

            let format = CStr::from_ptr(d_t_fmt_ptr).to_str().ok()?;
            #[cfg(test)]
            eprintln!("DEBUG: get_locale_format_string: raw format from nl_langinfo: '{}'", format);

            if format.is_empty() {
                #[cfg(test)]
                eprintln!("DEBUG: get_locale_format_string: format string is empty");
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

            // Print the actual format for debugging on macOS and other platforms
            eprintln!("DEBUG: Detected locale format: '{}'", format);
            eprintln!("DEBUG: Platform: {}", std::env::consts::OS);
            eprintln!("DEBUG: Arch: {}", std::env::consts::ARCH);

            // Check for environment variables that might affect locale
            for var in ["LC_ALL", "LC_TIME", "LANG"] {
                if let Ok(val) = std::env::var(var) {
                    eprintln!("DEBUG: {}={}", var, val);
                } else {
                    eprintln!("DEBUG: {} is not set", var);
                }
            }

            assert!(format.contains("%a"), "Format '{}' should contain abbreviated weekday (%a)", format);
            assert!(format.contains("%b"), "Format '{}' should contain abbreviated month (%b)", format);
            assert!(format.contains("%Y") || format.contains("%y"), "Format '{}' should contain year (%Y or %y)", format);
            assert!(format.contains("%Z"), "Format '{}' should contain timezone (%Z)", format);
        }

        #[test]
        fn test_locale_format_structure() {
            // Verify we're using actual locale format strings, not hardcoded ones
            let format = get_locale_default_format();

            // Print detailed debugging information
            eprintln!("DEBUG: Testing locale format structure");
            eprintln!("DEBUG: Format string: '{}'", format);
            eprintln!("DEBUG: Format length: {} characters", format.len());

            // The format should not be empty
            assert!(!format.is_empty(), "Locale format should not be empty, got: '{}'", format);

            // Check for date components with detailed output
            let date_components = ["%a", "%A", "%b", "%B", "%d", "%e"];
            let found_date_components: Vec<_> = date_components.iter()
                .filter(|&comp| format.contains(comp))
                .collect();
            eprintln!("DEBUG: Found date components: {:?}", found_date_components);

            let has_date_component = !found_date_components.is_empty();
            assert!(has_date_component,
                "Format '{}' should contain date components. Checked: {:?}, Found: {:?}",
                format, date_components, found_date_components);

            // Check for time components with detailed output
            let time_components = ["%H", "%I", "%k", "%l", "%r", "%R", "%T", "%X"];
            let found_time_components: Vec<_> = time_components.iter()
                .filter(|&comp| format.contains(comp))
                .collect();
            eprintln!("DEBUG: Found time components: {:?}", found_time_components);

            let has_time_component = !found_time_components.is_empty();
            assert!(has_time_component,
                "Format '{}' should contain time components. Checked: {:?}, Found: {:?}",
                format, time_components, found_time_components);

            // Additional debug: show raw locale format from system
            eprintln!("DEBUG: Checking raw system locale format...");
            if let Some(raw_format) = get_locale_format_string() {
                eprintln!("DEBUG: Raw system format: '{}'", raw_format);
                eprintln!("DEBUG: Raw format has timezone: {}", raw_format.contains("%Z"));
            } else {
                eprintln!("DEBUG: No raw system format available (using fallback)");
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
