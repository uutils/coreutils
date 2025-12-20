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
}

cfg_langinfo! {
    /// Cached result of locale time format detection
    static TIME_FORMAT_CACHE: OnceLock<bool> = OnceLock::new();

    /// Safe wrapper around libc setlocale
    fn set_time_locale() {
        unsafe {
            nix::libc::setlocale(nix::libc::LC_TIME, c"".as_ptr());
        }
    }

    /// Safe wrapper around libc nl_langinfo that returns `Option<String>`
    fn get_locale_info(item: nix::libc::nl_item) -> Option<String> {
        unsafe {
            let ptr = nix::libc::nl_langinfo(item);
            if ptr.is_null() {
                None
            } else {
                CStr::from_ptr(ptr).to_str().ok().map(String::from)
            }
        }
    }

    /// Internal function that performs the actual locale detection
    fn detect_12_hour_format() -> bool {
        // Helper function to check for 12-hour format indicators
        fn has_12_hour_indicators(format_str: &str) -> bool {
            const INDICATORS: &[&str] = &["%I", "%l", "%r"];
            INDICATORS.iter().any(|&indicator| format_str.contains(indicator))
        }

        // Helper function to check for 24-hour format indicators
        fn has_24_hour_indicators(format_str: &str) -> bool {
            const INDICATORS: &[&str] = &["%H", "%k", "%R", "%T"];
            INDICATORS.iter().any(|&indicator| format_str.contains(indicator))
        }

        // Set locale from environment variables (empty string = use LC_TIME/LANG env vars)
        set_time_locale();

        // Get locale format strings using safe wrappers
        let d_t_fmt = get_locale_info(nix::libc::D_T_FMT);
        let t_fmt_opt = get_locale_info(nix::libc::T_FMT);
        let t_fmt_ampm_opt = get_locale_info(nix::libc::T_FMT_AMPM);

        // Check D_T_FMT first
        if let Some(ref format) = d_t_fmt {
            // Check for 12-hour indicators first (higher priority)
            if has_12_hour_indicators(format) {
                return true;
            }

            // If we find 24-hour indicators, it's definitely not 12-hour
            if has_24_hour_indicators(format) {
                return false;
            }
        }

        // Also check the time-only format as a fallback
        if let Some(ref time_format) = t_fmt_opt {
            if has_12_hour_indicators(time_format) {
                return true;
            }
        }

        // Check if there's a specific 12-hour format defined
        if let Some(ref ampm_format) = t_fmt_ampm_opt {
            // If T_FMT_AMPM is non-empty and different from T_FMT, locale supports 12-hour
            if !ampm_format.is_empty() {
                if let Some(ref time_format) = t_fmt_opt {
                    if ampm_format != time_format {
                        return true;
                    }
                } else {
                    return true;
                }
            }
        }

        // Default to 24-hour format if we can't determine
        false
    }
}

cfg_langinfo! {
    /// Detects whether the current locale prefers 12-hour or 24-hour time format
    /// Results are cached for performance
    pub fn uses_12_hour_format() -> bool {
        *TIME_FORMAT_CACHE.get_or_init(detect_12_hour_format)
    }

    /// Cached default format string
    static DEFAULT_FORMAT_CACHE: OnceLock<&'static str> = OnceLock::new();

    /// Get the locale-appropriate default format string for date output
    /// This respects the locale's preference for 12-hour vs 24-hour time
    /// Results are cached for performance (following uucore patterns)
    pub fn get_locale_default_format() -> &'static str {
        DEFAULT_FORMAT_CACHE.get_or_init(|| {
            if uses_12_hour_format() {
                // Use 12-hour format with AM/PM
                "%a %b %e %r %Z %Y"
            } else {
                // Use 24-hour format
                "%a %b %e %X %Z %Y"
            }
        })
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
            let _ = uses_12_hour_format();
            let _ = get_locale_default_format();
        }

        #[test]
        fn test_default_format_contains_valid_codes() {
            let format = get_locale_default_format();
            assert!(format.contains("%a")); // abbreviated weekday
            assert!(format.contains("%b")); // abbreviated month
            assert!(format.contains("%Y")); // year
            assert!(format.contains("%Z")); // timezone
        }
    }
}
