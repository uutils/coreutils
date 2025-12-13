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

    /// Internal function that performs the actual locale detection
    fn detect_12_hour_format() -> bool {
        unsafe {
            // Set locale from environment variables (empty string = use LC_TIME/LANG env vars)
            libc::setlocale(libc::LC_TIME, c"".as_ptr());

        // Get the date/time format string from locale
        let d_t_fmt_ptr = libc::nl_langinfo(libc::D_T_FMT);
        if d_t_fmt_ptr.is_null() {
            return false;
        }

        let Ok(format) = CStr::from_ptr(d_t_fmt_ptr).to_str() else {
            return false;
        };

        // Check for 12-hour indicators first (higher priority)
        // %I = hour (01-12), %l = hour (1-12) space-padded, %r = 12-hour time with AM/PM
        if format.contains("%I") || format.contains("%l") || format.contains("%r") {
            return true;
        }

        // If we find 24-hour indicators, it's definitely not 12-hour
        // %H = hour (00-23), %k = hour (0-23) space-padded, %R = %H:%M, %T = %H:%M:%S
        if format.contains("%H")
            || format.contains("%k")
            || format.contains("%R")
            || format.contains("%T")
        {
            return false;
        }

        // Also check the time-only format as a fallback
        let t_fmt_ptr = libc::nl_langinfo(libc::T_FMT);
        let mut time_fmt_opt = None;
        if !t_fmt_ptr.is_null() {
            if let Ok(time_format) = CStr::from_ptr(t_fmt_ptr).to_str() {
                time_fmt_opt = Some(time_format);
                if time_format.contains("%I")
                    || time_format.contains("%l")
                    || time_format.contains("%r")
                {
                    return true;
                }
            }
        }

        // Check if there's a specific 12-hour format defined
        let t_fmt_ampm_ptr = libc::nl_langinfo(libc::T_FMT_AMPM);
        if !t_fmt_ampm_ptr.is_null() {
            if let Ok(ampm_format) = CStr::from_ptr(t_fmt_ampm_ptr).to_str() {
                // If T_FMT_AMPM is non-empty and different from T_FMT, locale supports 12-hour
                if !ampm_format.is_empty() {
                    if let Some(time_format) = time_fmt_opt {
                        if ampm_format != time_format {
                            return true;
                        }
                    } else {
                        return true;
                    }
                }
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
