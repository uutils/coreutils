// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (acronyms) CTYPE CLDR
// spell-checker:ignore (terms) Guillemets

//! Locale-specific quotation mark detection and mapping.
//!
//! This module provides functionality to determine appropriate quotation marks
//! based on the system's locale settings (LC_ALL, LC_CTYPE, or LANG).
//!
//! # Unicode Quotation Marks
//!
//! Different locales use different quotation mark conventions:
//! - U+0022 (") - ASCII double quote (English, default)
//! - U+00AB («) / U+00BB (») - Guillemets (French, Spanish, Russian)
//! - U+201E („) / U+201C (") - Low-9 and high quotes (German, Czech)
//! - U+300C (「) / U+300D (」) - Corner brackets (Japanese)
//! - U+201C (") / U+201D (") - Curly quotes (Chinese)
//!
//! # References
//!
//! - Unicode Standard Annex #14: Line Breaking Properties
//! - CLDR Locale Data: Quotation marks

use std::env;

/// Returns locale-specific opening and closing quotation marks.
///
/// This function examines the environment variables in the following order:
/// 1. `LC_ALL` - Overrides all other locale settings
/// 2. `LC_CTYPE` - Controls character classification and case conversion
/// 3. `LANG` - Default locale setting
///
/// # Returns
///
/// A tuple `(opening_quote, closing_quote)` appropriate for the detected locale.
/// Returns `('"', '"')` (ASCII double quotes) as a safe default for unknown locales.
///
/// # Examples
///
/// ```ignore
/// use uucore::quoting_style::locale_quotes::get_locale_quote_chars;
///
/// // With default or English locale
/// assert_eq!(get_locale_quote_chars(), ('"', '"'));
/// ```
///
/// # Safety
///
/// This function only reads environment variables and performs string matching.
/// All returned characters are valid Unicode scalar values.
pub fn get_locale_quote_chars() -> (char, char) {
    // Try environment variables in order of precedence
    let locale = env::var("LC_ALL")
        .or_else(|_| env::var("LC_CTYPE"))
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

    map_locale_to_quotes(locale)
}

/// Maps a locale identifier to appropriate quotation marks.
///
/// # Arguments
///
/// * `locale` - A locale identifier (e.g., "fr_FR", "de_DE", "ja_JP")
///
/// # Returns
///
/// A tuple of (opening_quote, closing_quote) characters.
///
/// # Locale Coverage
///
/// This function covers major language families and follows Unicode/CLDR conventions:
/// - Romance languages (French, Spanish, Portuguese, Italian): Guillemets « »
/// - Germanic languages (German, Czech, Slovak): Low-9 and high quotes „ "
/// - Slavic languages (Russian, Ukrainian, Bulgarian): Guillemets « »
/// - CJK (Japanese): Corner brackets 「 」
/// - CJK (Chinese, Korean): Curly quotes " "
/// - English and default: ASCII double quotes " "
#[allow(clippy::match_like_matches_macro)] // Clearer as explicit match for documentation
fn map_locale_to_quotes(locale: &str) -> (char, char) {
    // Extract language code (first 2-3 characters before underscore)
    let lang = locale.split('_').next().unwrap_or(locale);

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
