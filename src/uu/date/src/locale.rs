// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore charsets euctw

//! Locale detection for time format preferences

use std::borrow::Cow;

/// Format used when the locale does not provide a date/time format.
const POSIX_DEFAULT_FORMAT: &[u8] = b"%a %b %e %X %Z %Y";

/// The specifiers that expand to a locale format, and their C locale formats,
/// used without `nl_langinfo` or when the locale leaves one empty.
const LOCALE_FORMAT_SPECIFIERS: [&str; 3] = ["x", "X", "r"];
const POSIX_LOCALE_FORMATS: [&str; 3] = ["%m/%d/%y", "%H:%M:%S", "%I:%M:%S %p"];
const POSIX_AMPM_MARKERS: [&str; 2] = ["AM", "PM"];

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

// `nl_langinfo`, with the POSIX items `D_FMT`, `T_FMT` and `T_FMT_AMPM` behind
// `%x`, `%X` and `%r`, exists on every unix but Android, Cygwin and Redox;
// `cfg_nl_langinfo!(else ...)` gates the items for the other platforms.
macro_rules! cfg_nl_langinfo {
    (else $($item:item)*) => {
        $(
            #[cfg(any(
                not(unix),
                target_os = "android",
                target_os = "cygwin",
                target_os = "redox"
            ))]
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
    use std::sync::LazyLock;

    #[cfg(test)]
    use std::sync::Mutex;

    /// Mutex to serialize setlocale() calls during tests.
    ///
    /// setlocale() is process-global, so parallel tests that call it can
    /// interfere with each other. This mutex ensures only one test accesses
    /// locale functions at a time.
    #[cfg(test)]
    static LOCALE_MUTEX: Mutex<()> = Mutex::new(());

    /// The locale's `D_FMT`, `T_FMT` and `T_FMT_AMPM`, read once.
    static LOCALE_FORMATS: LazyLock<[&'static str; 3]> = LazyLock::new(|| {
        let items = [libc::D_FMT, libc::T_FMT, libc::T_FMT_AMPM];
        let mut formats = POSIX_LOCALE_FORMATS;
        for (format, item) in formats.iter_mut().zip(items) {
            if let Some(value) = langinfo(item)
                .and_then(|value| String::from_utf8(value).ok())
                .filter(|value| !value.is_empty())
            {
                *format = value.leak();
            }
        }
        formats
    });

    /// The locale's `AM_STR` and `PM_STR`, read once. Either may be empty.
    static LOCALE_AMPM_MARKERS: LazyLock<[&'static str; 2]> = LazyLock::new(|| {
        let mut markers = POSIX_AMPM_MARKERS;
        for (marker, item) in markers.iter_mut().zip([libc::AM_STR, libc::PM_STR]) {
            if let Some(value) = langinfo(item).and_then(|value| String::from_utf8(value).ok()) {
                *marker = value.leak();
            }
        }
        markers
    });

    /// Reads a langinfo item of the `LC_TIME` locale set in the environment.
    fn langinfo(item: libc::nl_item) -> Option<Vec<u8>> {
        // In tests, acquire mutex to prevent race conditions with setlocale()
        // which is process-global and not thread-safe
        #[cfg(test)]
        let _lock = LOCALE_MUTEX.lock().unwrap();

        // SAFETY: the locale name is a NUL-terminated literal. nl_langinfo
        // returns a NUL-terminated string (or null, checked first) that stays
        // valid until the next setlocale or nl_langinfo call; it is copied out
        // before this function returns. date makes these calls from its main
        // thread only, to fill once-initialized caches, and tests hold
        // `LOCALE_MUTEX`.
        unsafe {
            libc::setlocale(libc::LC_TIME, c"".as_ptr());
            let ptr = libc::nl_langinfo(item);
            (!ptr.is_null()).then(|| CStr::from_ptr(ptr).to_bytes().to_vec())
        }
    }
}

cfg_nl_langinfo! { else
    static LOCALE_FORMATS: [&str; 3] = POSIX_LOCALE_FORMATS;
    static LOCALE_AMPM_MARKERS: [&str; 2] = POSIX_AMPM_MARKERS;
}

/// Returns the locale's format for `%x`, `%X` or `%r`, given the specifier
/// without `%`, and `None` for any other specifier.
pub fn get_locale_format(specifier: &str) -> Option<&'static str> {
    let index = LOCALE_FORMAT_SPECIFIERS
        .iter()
        .position(|known| *known == specifier)?;
    Some(LOCALE_FORMATS[index])
}

/// Returns the locale's AM or PM marker for `%p`, lowercased for `%P`, given
/// the specifier without `%`, and `None` for any other specifier.
pub fn get_locale_ampm_marker(specifier: &str, is_pm: bool) -> Option<Cow<'static, str>> {
    let lowercase = match specifier {
        "p" => false,
        "P" => true,
        _ => return None,
    };
    let marker = LOCALE_AMPM_MARKERS[usize::from(is_pm)];
    Some(if lowercase {
        Cow::Owned(marker.to_lowercase())
    } else {
        Cow::Borrowed(marker)
    })
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
        langinfo(DATE_FMT).filter(|format| !format.is_empty())
    }
}

cfg_langinfo! { else
    /// On platforms without `_DATE_FMT`, fall back to the POSIX format
    pub fn get_locale_default_format() -> &'static [u8] {
        POSIX_DEFAULT_FORMAT
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
