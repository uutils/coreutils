// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (acronyms) CTYPE

//! Locale-specific time format detection and mapping.
//!
//! This module provides functionality to determine appropriate time/date format strings
//! based on the system's locale settings (LC_ALL, LC_TIME, or LANG).
//!
//! # Time Format Structure
//!
//! Different locales use different date/time conventions:
//! - Date component ordering: DMY (European), MDY (American), YMD (Asian)
//! - Time format: 24-hour (military) vs 12-hour (AM/PM)
//! - Month representation: Abbreviated names vs numbers
//!
//! # Format String Pairs
//!
//! ls uses two format strings:
//! - Recent files (modified within last 6 months): Shows month, day, and time
//! - Older files (modified more than 6 months ago): Shows month, day, and year
//!
//! # References
//!
//! - POSIX locale specifications
//! - GNU ls source code (ls.c, timespec format)
//! - Common Locale Data Repository (CLDR)

use std::env;

/// Returns locale-specific time format strings for ls --time-style=locale.
///
/// This function examines the environment variables in the following order:
/// 1. `LC_ALL` - Overrides all other locale settings
/// 2. `LC_TIME` - Controls time and date formatting
/// 3. `LANG` - Default locale setting
///
/// # Returns
///
/// A tuple of `(recent_format, older_format_option)`:
/// - `recent_format`: Format for files modified within last 6 months (shows time)
/// - `older_format`: Optional different format for older files (shows year)
///
/// Returns English/POSIX default format for unknown locales.
///
/// # Format Code Reference
///
/// Common strftime codes used:
/// - `%b` - Abbreviated month name
/// - `%d` - Day of month (01-31), zero-padded
/// - `%e` - Day of month ( 1-31), space-padded
/// - `%m` - Month number (01-12)
/// - `%Y` - Year (4 digits)
/// - `%H` - Hour (00-23), 24-hour format
/// - `%I` - Hour (01-12), 12-hour format
/// - `%M` - Minute (00-59)
/// - `%p` - AM/PM designator
///
/// # Examples
///
/// ```ignore
/// use uu_ls::locale_time::get_locale_time_formats;
///
/// // With default or English locale
/// let (recent, older) = get_locale_time_formats();
/// assert_eq!(recent, "%b %e %H:%M");
/// assert_eq!(older, Some("%b %e  %Y"));
/// ```
///
/// # Safety
///
/// This function only reads environment variables and performs string matching.
/// All returned format strings are valid strftime format strings.
pub fn get_locale_time_formats() -> (&'static str, Option<&'static str>) {
    // Try environment variables in order of precedence
    let locale = env::var("LC_ALL")
        .or_else(|_| env::var("LC_TIME"))
        .or_else(|_| env::var("LANG"))
        .unwrap_or_default();

    // Parse locale identifier: lang_COUNTRY.encoding@modifier -> lang_COUNTRY
    // Examples: fr_FR.UTF-8, de_DE@euro, en_US, ja_JP.eucJP
    let locale = locale
        .split('.')
        .next()
        .unwrap_or(&locale)
        .split('@')
        .next()
        .unwrap_or("");

    map_locale_to_time_formats(locale)
}

/// Maps a locale identifier to appropriate time format strings.
///
/// # Arguments
///
/// * `locale` - A locale identifier (e.g., "fr_FR", "de_DE", "ja_JP")
///
/// # Returns
///
/// A tuple of (recent_format, older_format) strings.
///
/// # Locale Format Conventions
///
/// This function follows common date/time formatting conventions:
/// - **European (DMY)**: Day-Month-Year ordering, 24-hour time
/// - **American (MDY)**: Month-Day-Year ordering, preference varies
/// - **Asian (YMD)**: Year-Month-Day ordering, 24-hour time
/// - **Default/POSIX**: Follows traditional Unix ls format
#[allow(clippy::match_like_matches_macro)] // Clearer as explicit match for documentation
fn map_locale_to_time_formats(locale: &str) -> (&'static str, Option<&'static str>) {
    // Extract language code (first 2-3 characters before underscore)
    let lang = locale.split('_').next().unwrap_or(locale);

    match lang {
        // English locales (US, GB, AU, etc.) - MDY or DMY
        // US: Month Day, Year - 12-hour time preference but 24-hour is standard for ls
        "en" => {
            if locale.starts_with("en_US") {
                // American: MDY format
                ("%b %e %H:%M", Some("%b %e  %Y"))
            } else {
                // British/Commonwealth: DMY format
                ("%d %b %H:%M", Some("%d %b  %Y"))
            }
        }

        // Romance languages - typically DMY with 24-hour time
        // French: DD Mon YYYY HH:MM
        "fr" => ("%e %b %H:%M", Some("%e %b  %Y")),

        // Spanish: DD Mon YYYY HH:MM
        "es" => ("%e %b %H:%M", Some("%e %b  %Y")),

        // Italian: DD Mon YYYY HH:MM
        "it" => ("%e %b %H:%M", Some("%e %b  %Y")),

        // Portuguese: DD/MM HH:MM and DD/MM/YYYY
        "pt" => ("%d/%m %H:%M", Some("%d/%m/%Y")),

        // Catalan: DD Mon YYYY HH:MM
        "ca" => ("%e %b %H:%M", Some("%e %b  %Y")),

        // Romanian: DD.MM.YYYY HH:MM
        "ro" => ("%d.%m %H:%M", Some("%d.%m.%Y")),

        // Germanic languages - typically DMY with 24-hour time
        // German: DD. Mon YYYY HH:MM
        "de" => ("%e. %b %H:%M", Some("%e. %b  %Y")),

        // Dutch: DD Mon YYYY HH:MM
        "nl" => ("%e %b %H:%M", Some("%e %b  %Y")),

        // Danish: DD-MM HH:MM and DD-MM-YYYY
        "da" => ("%d-%m %H:%M", Some("%d-%m-%Y")),

        // Swedish: DD Mon HH:MM and DD Mon YYYY
        "sv" => ("%e %b %H:%M", Some("%e %b %Y")),

        // Norwegian: DD. Mon HH:MM and DD. Mon YYYY
        "no" | "nb" | "nn" => ("%e. %b %H:%M", Some("%e. %b %Y")),

        // Finnish: DD.MM. HH:MM and DD.MM.YYYY
        "fi" => ("%d.%m. %H:%M", Some("%d.%m.%Y")),

        // Slavic languages - typically DMY with 24-hour time
        // Russian: DD Mon HH:MM and DD Mon YYYY
        "ru" => ("%e %b %H:%M", Some("%e %b %Y")),

        // Polish: DD Mon HH:MM and DD Mon YYYY
        "pl" => ("%e %b %H:%M", Some("%e %b %Y")),

        // Czech: DD. Mon HH:MM and DD. Mon YYYY
        "cs" => ("%e. %b %H:%M", Some("%e. %b %Y")),

        // Slovak: DD. Mon HH:MM and DD. Mon YYYY
        "sk" => ("%e. %b %H:%M", Some("%e. %b %Y")),

        // Ukrainian: DD Mon HH:MM and DD Mon YYYY
        "uk" => ("%e %b %H:%M", Some("%e %b %Y")),

        // Bulgarian: DD.MM. HH:MM and DD.MM.YYYY
        "bg" => ("%d.%m. %H:%M", Some("%d.%m.%Y")),

        // Serbian: DD. Mon HH:MM and DD. Mon YYYY
        "sr" => ("%e. %b %H:%M", Some("%e. %b %Y")),

        // Croatian: DD. Mon HH:MM and DD. Mon YYYY
        "hr" => ("%e. %b %H:%M", Some("%e. %b %Y")),

        // Asian languages - typically YMD with 24-hour time
        // Japanese: MM月DD日 HH:MM and YYYY年MM月DD日
        "ja" => ("%m月%d日 %H:%M", Some("%Y年%m月%d日")),

        // Chinese (Simplified and Traditional): MM月DD日 HH:MM and YYYY年MM月DD日
        "zh" => {
            if locale.contains("TW") || locale.contains("HK") {
                // Traditional Chinese: might use different format
                ("%m月%d日 %H:%M", Some("%Y年%m月%d日"))
            } else {
                // Simplified Chinese
                ("%m月%d日 %H:%M", Some("%Y年%m月%d日"))
            }
        }

        // Korean: MM. DD. HH:MM and YYYY. MM. DD.
        "ko" => ("%m. %d. %H:%M", Some("%Y. %m. %d.")),

        // Other European languages
        // Greek: DD Mon HH:MM and DD Mon YYYY
        "el" => ("%e %b %H:%M", Some("%e %b %Y")),

        // Turkish: DD Mon HH:MM and DD Mon YYYY
        "tr" => ("%e %b %H:%M", Some("%e %b %Y")),

        // Hungarian: Mon DD. HH:MM and YYYY Mon DD.
        "hu" => ("%b %e. %H:%M", Some("%Y %b %e.")),

        // Baltic languages
        // Estonian: DD. Mon HH:MM and DD. Mon YYYY
        "et" => ("%e. %b %H:%M", Some("%e. %b %Y")),

        // Latvian: DD. Mon HH:MM and DD. Mon YYYY
        "lv" => ("%e. %b %H:%M", Some("%e. %b %Y")),

        // Lithuanian: Mon DD HH:MM and YYYY Mon DD
        "lt" => ("%b %e %H:%M", Some("%Y %b %e")),

        // Arabic/Hebrew/Persian - RTL languages, typically DMY
        // Note: Month names will still be English due to %b limitation
        "ar" | "he" | "fa" => ("%e %b %H:%M", Some("%e %b %Y")),

        // Thai: DD Mon HH:MM and DD Mon YYYY
        "th" => ("%e %b %H:%M", Some("%e %b %Y")),

        // Vietnamese: Mon DD HH:MM and Mon DD YYYY
        // Note: Vietnamese traditionally uses DMY, but GNU ls uses MDY format
        "vi" => ("%b %e %H:%M", Some("%b %e  %Y")),

        // Indonesian/Malay: DD Mon HH:MM and DD Mon YYYY
        "id" | "ms" => ("%d %b %H:%M", Some("%d %b %Y")),

        // C, POSIX, and default - Traditional Unix ls format
        "C" | "POSIX" | "" => ("%b %e %H:%M", Some("%b %e  %Y")),

        // Default fallback - POSIX/English format
        _ => ("%b %e %H:%M", Some("%b %e  %Y")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_english_us() {
        let (recent, older) = map_locale_to_time_formats("en_US");
        assert_eq!(recent, "%b %e %H:%M");
        assert_eq!(older, Some("%b %e  %Y"));
    }

    #[test]
    fn test_english_gb() {
        let (recent, older) = map_locale_to_time_formats("en_GB");
        assert_eq!(recent, "%d %b %H:%M");
        assert_eq!(older, Some("%d %b  %Y"));
    }

    #[test]
    fn test_french() {
        let (recent, older) = map_locale_to_time_formats("fr_FR");
        assert_eq!(recent, "%e %b %H:%M");
        assert_eq!(older, Some("%e %b  %Y"));
    }

    #[test]
    fn test_german() {
        let (recent, older) = map_locale_to_time_formats("de_DE");
        assert_eq!(recent, "%e. %b %H:%M");
        assert_eq!(older, Some("%e. %b  %Y"));
    }

    #[test]
    fn test_japanese() {
        let (recent, older) = map_locale_to_time_formats("ja_JP");
        assert_eq!(recent, "%m月%d日 %H:%M");
        assert_eq!(older, Some("%Y年%m月%d日"));
    }

    #[test]
    fn test_chinese() {
        let (recent, older) = map_locale_to_time_formats("zh_CN");
        assert_eq!(recent, "%m月%d日 %H:%M");
        assert_eq!(older, Some("%Y年%m月%d日"));
    }

    #[test]
    fn test_default_fallback() {
        let (recent, older) = map_locale_to_time_formats("unknown_XX");
        assert_eq!(recent, "%b %e %H:%M");
        assert_eq!(older, Some("%b %e  %Y"));
    }

    #[test]
    fn test_posix() {
        let (recent, older) = map_locale_to_time_formats("POSIX");
        assert_eq!(recent, "%b %e %H:%M");
        assert_eq!(older, Some("%b %e  %Y"));
    }

    #[test]
    fn test_vietnamese() {
        let (recent, older) = map_locale_to_time_formats("vi_VN");
        assert_eq!(recent, "%b %e %H:%M");
        assert_eq!(older, Some("%b %e  %Y"));
    }
}
