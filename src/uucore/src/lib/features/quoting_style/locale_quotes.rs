// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (acronyms) CTYPE CLDR
// spell-checker:ignore (terms) Guillemets

//! Locale-specific quotation mark detection and mapping.
//!
//! Provides functionality to determine appropriate quotation marks based on
//! the system's locale settings (LC_ALL, LC_CTYPE, or LANG).
//!
//! # Unicode Quotation Marks
//!
//! - U+0022 (") - ASCII double quote (English, default)
//! - U+00AB («) / U+00BB (») - Guillemets (French, Spanish, Russian)
//! - U+201E („) / U+201C (") - Low-9 and high quotes (German, Czech)
//! - U+300C (「) / U+300D (」) - Corner brackets (Japanese)
//! - U+201C (") / U+201D (") - Curly quotes (Chinese)

use std::sync::OnceLock;

/// Cached locale quote characters to avoid repeated environment variable lookups.
static LOCALE_QUOTES: OnceLock<(char, char)> = OnceLock::new();

/// Returns locale-specific opening and closing quotation marks.
///
/// The result is cached on first call to avoid repeated environment variable lookups.
///
/// # Returns
///
/// A tuple `(opening_quote, closing_quote)` appropriate for the detected locale.
/// Returns `('"', '"')` (ASCII double quotes) as a safe default for unknown locales.
pub fn get_locale_quote_chars() -> (char, char) {
    *LOCALE_QUOTES.get_or_init(|| {
        let locale_str = std::env::var("LC_ALL")
            .or_else(|_| std::env::var("LC_CTYPE"))
            .or_else(|_| std::env::var("LANG"))
            .unwrap_or_default();

        // Parse locale identifier: lang_COUNTRY.encoding@modifier -> lang_COUNTRY
        let locale_end = locale_str.find('.').unwrap_or(locale_str.len());
        let locale = if let Some(at_pos) = locale_str.find('@') {
            &locale_str[..at_pos.min(locale_end)]
        } else {
            &locale_str[..locale_end]
        };

        map_locale_to_quotes(locale)
    })
}

/// Maps a locale identifier to appropriate quotation marks.
///
/// * `locale` - A locale identifier (e.g., "fr_FR", "de_DE", "ja_JP")
#[allow(clippy::match_like_matches_macro)]
fn map_locale_to_quotes(locale: &str) -> (char, char) {
    let lang = if let Some(underscore_pos) = locale.find('_') {
        &locale[..underscore_pos.min(3)]
    } else {
        locale
    };

    match lang {
        // Romance languages - Guillemets (U+00AB, U+00BB)
        // French, Spanish, Catalan, Portuguese, Italian, Romanian
        "fr" | "es" | "ca" | "pt" | "it" | "ro" => ('\u{00AB}', '\u{00BB}'),

        // Germanic languages with low-9 quotes (U+201E, U+201C)
        // German, Czech, Slovak, Estonian, Latvian, Lithuanian
        "de" | "cs" | "sk" | "et" | "lv" | "lt" => ('\u{201E}', '\u{201C}'),

        // Slavic languages - Guillemets (U+00AB, U+00BB)
        // Russian, Ukrainian, Bulgarian, Serbian, Belarusian
        "ru" | "uk" | "bg" | "sr" | "be" => ('\u{00AB}', '\u{00BB}'),

        // Polish - Low-9 double quotes (U+201E, U+201D)
        "pl" => ('\u{201E}', '\u{201D}'),

        // Japanese - Corner brackets (U+300C, U+300D)
        "ja" => ('\u{300C}', '\u{300D}'),

        // Chinese (Simplified and Traditional) - CJK curly quotes (U+201C, U+201D)
        // Also Korean
        "zh" | "ko" => ('\u{201C}', '\u{201D}'),

        // Dutch, Danish, Norwegian, Finnish - Using ASCII double quotes for compatibility
        "nl" | "da" | "no" | "nb" | "nn" | "fi" => ('"', '"'),

        // Swedish - Using ASCII double quotes for compatibility
        "sv" => ('"', '"'),

        // Greek - ASCII-like quotes
        "el" => ('"', '"'),

        // Turkish - ASCII quotes
        "tr" => ('"', '"'),

        // Arabic/Hebrew/Persian - fallback to ASCII for safety
        "ar" | "fa" | "he" => ('"', '"'),

        // C, POSIX, and English locales - ASCII double quotes (U+0022)
        "C" | "POSIX" | "en" => ('"', '"'),

        // Default fallback - ASCII double quotes
        _ => ('"', '"'),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    // Helper function to safely test with environment variables
    fn with_locale_vars<F>(lc_all: Option<&str>, lc_ctype: Option<&str>, lang: Option<&str>, f: F)
    where
        F: FnOnce(),
    {
        // Save original values
        let orig_lc_all = env::var("LC_ALL").ok();
        let orig_lc_ctype = env::var("LC_CTYPE").ok();
        let orig_lang = env::var("LANG").ok();

        // Clear all locale vars first
        unsafe {
            env::remove_var("LC_ALL");
            env::remove_var("LC_CTYPE");
            env::remove_var("LANG");
        }

        // Set test values
        unsafe {
            if let Some(val) = lc_all {
                env::set_var("LC_ALL", val);
            }
            if let Some(val) = lc_ctype {
                env::set_var("LC_CTYPE", val);
            }
            if let Some(val) = lang {
                env::set_var("LANG", val);
            }
        }

        // Run test
        f();

        // Restore original values
        unsafe {
            env::remove_var("LC_ALL");
            env::remove_var("LC_CTYPE");
            env::remove_var("LANG");
            if let Some(val) = orig_lc_all {
                env::set_var("LC_ALL", val);
            }
            if let Some(val) = orig_lc_ctype {
                env::set_var("LC_CTYPE", val);
            }
            if let Some(val) = orig_lang {
                env::set_var("LANG", val);
            }
        }
    }

    #[test]
    fn test_map_locale_to_quotes_romance_languages() {
        // French, Spanish, Catalan, Portuguese, Italian, Romanian use Guillemets
        let guillemets = ('\u{00AB}', '\u{00BB}');
        assert_eq!(map_locale_to_quotes("fr"), guillemets);
        assert_eq!(map_locale_to_quotes("es"), guillemets);
        assert_eq!(map_locale_to_quotes("ca"), guillemets);
        assert_eq!(map_locale_to_quotes("pt"), guillemets);
        assert_eq!(map_locale_to_quotes("it"), guillemets);
        assert_eq!(map_locale_to_quotes("ro"), guillemets);

        // Test with full locale strings
        assert_eq!(map_locale_to_quotes("fr_FR"), guillemets);
        assert_eq!(map_locale_to_quotes("es_ES"), guillemets);
    }

    #[test]
    fn test_map_locale_to_quotes_germanic_languages() {
        // German, Czech, Slovak, Estonian, Latvian, Lithuanian use Low-9 and high quotes
        let low9_quotes = ('\u{201E}', '\u{201C}');
        assert_eq!(map_locale_to_quotes("de"), low9_quotes);
        assert_eq!(map_locale_to_quotes("cs"), low9_quotes);
        assert_eq!(map_locale_to_quotes("sk"), low9_quotes);
        assert_eq!(map_locale_to_quotes("et"), low9_quotes);
        assert_eq!(map_locale_to_quotes("lv"), low9_quotes);
        assert_eq!(map_locale_to_quotes("lt"), low9_quotes);

        // Test with full locale strings
        assert_eq!(map_locale_to_quotes("de_DE"), low9_quotes);
        assert_eq!(map_locale_to_quotes("cs_CZ"), low9_quotes);
    }

    #[test]
    fn test_map_locale_to_quotes_slavic_languages() {
        // Russian, Ukrainian, Bulgarian, Serbian, Belarusian use Guillemets
        let guillemets = ('\u{00AB}', '\u{00BB}');
        assert_eq!(map_locale_to_quotes("ru"), guillemets);
        assert_eq!(map_locale_to_quotes("uk"), guillemets);
        assert_eq!(map_locale_to_quotes("bg"), guillemets);
        assert_eq!(map_locale_to_quotes("sr"), guillemets);
        assert_eq!(map_locale_to_quotes("be"), guillemets);

        // Test with full locale strings
        assert_eq!(map_locale_to_quotes("ru_RU"), guillemets);
        assert_eq!(map_locale_to_quotes("uk_UA"), guillemets);
    }

    #[test]
    fn test_map_locale_to_quotes_polish() {
        // Polish uses Low-9 double quotes
        let polish_quotes = ('\u{201E}', '\u{201D}');
        assert_eq!(map_locale_to_quotes("pl"), polish_quotes);
        assert_eq!(map_locale_to_quotes("pl_PL"), polish_quotes);
    }

    #[test]
    fn test_map_locale_to_quotes_japanese() {
        // Japanese uses Corner brackets
        let corner_brackets = ('\u{300C}', '\u{300D}');
        assert_eq!(map_locale_to_quotes("ja"), corner_brackets);
        assert_eq!(map_locale_to_quotes("ja_JP"), corner_brackets);
    }

    #[test]
    fn test_map_locale_to_quotes_cjk_curly() {
        // Chinese and Korean use CJK curly quotes
        let curly_quotes = ('\u{201C}', '\u{201D}');
        assert_eq!(map_locale_to_quotes("zh"), curly_quotes);
        assert_eq!(map_locale_to_quotes("ko"), curly_quotes);

        // Test with full locale strings
        assert_eq!(map_locale_to_quotes("zh_CN"), curly_quotes);
        assert_eq!(map_locale_to_quotes("zh_TW"), curly_quotes);
        assert_eq!(map_locale_to_quotes("ko_KR"), curly_quotes);
    }

    #[test]
    fn test_map_locale_to_quotes_nordic_and_english() {
        // Dutch, Danish, Norwegian, Finnish, Swedish use ASCII double quotes
        let ascii_quotes = ('"', '"');
        assert_eq!(map_locale_to_quotes("nl"), ascii_quotes);
        assert_eq!(map_locale_to_quotes("da"), ascii_quotes);
        assert_eq!(map_locale_to_quotes("no"), ascii_quotes);
        assert_eq!(map_locale_to_quotes("nb"), ascii_quotes);
        assert_eq!(map_locale_to_quotes("nn"), ascii_quotes);
        assert_eq!(map_locale_to_quotes("fi"), ascii_quotes);
        assert_eq!(map_locale_to_quotes("sv"), ascii_quotes);
        assert_eq!(map_locale_to_quotes("en"), ascii_quotes);

        // Test with full locale strings
        assert_eq!(map_locale_to_quotes("en_US"), ascii_quotes);
        assert_eq!(map_locale_to_quotes("sv_SE"), ascii_quotes);
    }

    #[test]
    fn test_map_locale_to_quotes_other_languages() {
        // Greek, Turkish, Arabic, Hebrew, Persian use ASCII quotes
        let ascii_quotes = ('"', '"');
        assert_eq!(map_locale_to_quotes("el"), ascii_quotes);
        assert_eq!(map_locale_to_quotes("tr"), ascii_quotes);
        assert_eq!(map_locale_to_quotes("ar"), ascii_quotes);
        assert_eq!(map_locale_to_quotes("fa"), ascii_quotes);
        assert_eq!(map_locale_to_quotes("he"), ascii_quotes);
    }

    #[test]
    fn test_map_locale_to_quotes_c_posix() {
        // C and POSIX locales use ASCII double quotes
        let ascii_quotes = ('"', '"');
        assert_eq!(map_locale_to_quotes("C"), ascii_quotes);
        assert_eq!(map_locale_to_quotes("POSIX"), ascii_quotes);
    }

    #[test]
    fn test_map_locale_to_quotes_unknown() {
        // Unknown locales default to ASCII double quotes
        let ascii_quotes = ('"', '"');
        assert_eq!(map_locale_to_quotes("unknown"), ascii_quotes);
        assert_eq!(map_locale_to_quotes("xyz"), ascii_quotes);
        assert_eq!(map_locale_to_quotes(""), ascii_quotes);
    }

    #[test]
    fn test_map_locale_with_encoding_and_modifiers() {
        // Test that encoding and modifiers are properly stripped
        let guillemets = ('\u{00AB}', '\u{00BB}');
        assert_eq!(map_locale_to_quotes("fr_FR.UTF-8"), guillemets);
        assert_eq!(map_locale_to_quotes("fr_FR@euro"), guillemets);
        assert_eq!(map_locale_to_quotes("fr_FR.UTF-8@euro"), guillemets);

        let low9_quotes = ('\u{201E}', '\u{201C}');
        assert_eq!(map_locale_to_quotes("de_DE.UTF-8"), low9_quotes);
    }

    #[test]
    fn test_get_locale_quote_chars_with_lc_all() {
        with_locale_vars(
            Some("fr_FR.UTF-8"),
            Some("de_DE.UTF-8"),
            Some("en_US.UTF-8"),
            || {
                // LC_ALL should take precedence
                let guillemets = ('\u{00AB}', '\u{00BB}');
                assert_eq!(get_locale_quote_chars(), guillemets);
            },
        );
    }

    #[test]
    fn test_get_locale_quote_chars_with_lc_ctype() {
        with_locale_vars(None, Some("de_DE.UTF-8"), Some("en_US.UTF-8"), || {
            // LC_CTYPE should be used when LC_ALL is not set
            let low9_quotes = ('\u{201E}', '\u{201C}');
            assert_eq!(get_locale_quote_chars(), low9_quotes);
        });
    }

    #[test]
    fn test_get_locale_quote_chars_with_lang() {
        with_locale_vars(None, None, Some("ja_JP.UTF-8"), || {
            // LANG should be used when LC_ALL and LC_CTYPE are not set
            let corner_brackets = ('\u{300C}', '\u{300D}');
            assert_eq!(get_locale_quote_chars(), corner_brackets);
        });
    }

    #[test]
    fn test_get_locale_quote_chars_no_env_vars() {
        with_locale_vars(None, None, None, || {
            // Should default to ASCII double quotes when no locale vars are set
            let ascii_quotes = ('"', '"');
            assert_eq!(get_locale_quote_chars(), ascii_quotes);
        });
    }

    #[test]
    fn test_get_locale_quote_chars_precedence() {
        // Test that LC_ALL > LC_CTYPE > LANG
        with_locale_vars(Some("fr_FR"), Some("de_DE"), Some("en_US"), || {
            let guillemets = ('\u{00AB}', '\u{00BB}');
            assert_eq!(get_locale_quote_chars(), guillemets);
        });

        with_locale_vars(None, Some("de_DE"), Some("en_US"), || {
            let low9_quotes = ('\u{201E}', '\u{201C}');
            assert_eq!(get_locale_quote_chars(), low9_quotes);
        });

        with_locale_vars(None, None, Some("en_US"), || {
            let ascii_quotes = ('"', '"');
            assert_eq!(get_locale_quote_chars(), ascii_quotes);
        });
    }

    #[test]
    fn test_get_locale_quote_chars_with_c_locale() {
        with_locale_vars(Some("C"), None, None, || {
            let ascii_quotes = ('"', '"');
            assert_eq!(get_locale_quote_chars(), ascii_quotes);
        });

        with_locale_vars(Some("POSIX"), None, None, || {
            let ascii_quotes = ('"', '"');
            assert_eq!(get_locale_quote_chars(), ascii_quotes);
        });
    }
}
