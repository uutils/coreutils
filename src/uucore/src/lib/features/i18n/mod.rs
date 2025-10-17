// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::sync::OnceLock;

use icu_locale::{Locale, locale};

#[cfg(feature = "i18n-collator")]
pub mod collator;
#[cfg(feature = "i18n-datetime")]
pub mod datetime;
#[cfg(feature = "i18n-decimal")]
pub mod decimal;

/// The encoding specified by the locale, if specified
/// Currently only supports ASCII and UTF-8 for the sake of simplicity.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum UEncoding {
    Ascii,
    Utf8,
}

pub const DEFAULT_LOCALE: Locale = locale!("en-US-posix");

/// Look at 3 environment variables in the following order
///
/// 1. LC_ALL
/// 2. `locale_name`
/// 3. LANG
///
/// Or fallback on Posix locale, with ASCII encoding.
fn get_locale_from_env(locale_name: &str) -> (Locale, UEncoding) {
    let locale_var = ["LC_ALL", locale_name, "LANG"]
        .iter()
        .find_map(|&key| std::env::var(key).ok());

    if let Some(locale_var_str) = locale_var {
        let mut split = locale_var_str.split(&['.', '@']);

        if let Some(simple) = split.next() {
            // Naively convert the locale name to BCP47 tag format.
            //
            // See https://en.wikipedia.org/wiki/IETF_language_tag
            let bcp47 = simple.replace('_', "-");
            let locale = Locale::try_from_str(&bcp47).unwrap_or(DEFAULT_LOCALE);

            // If locale parsing failed, parse the encoding part of the
            // locale. Treat the special case of the given locale being "C"
            // which becomes the default locale.
            let encoding = if (locale != DEFAULT_LOCALE || bcp47 == "C")
                && split
                    .next()
                    .is_some_and(|enc| enc.to_lowercase() == "utf-8")
            {
                UEncoding::Utf8
            } else {
                UEncoding::Ascii
            };
            return (locale, encoding);
        }
    }
    // Default POSIX locale representing LC_ALL=C
    (DEFAULT_LOCALE, UEncoding::Ascii)
}

/// Get the collating locale from the environment
fn get_collating_locale() -> &'static (Locale, UEncoding) {
    static COLLATING_LOCALE: OnceLock<(Locale, UEncoding)> = OnceLock::new();

    COLLATING_LOCALE.get_or_init(|| get_locale_from_env("LC_COLLATE"))
}

/// Get the time locale from the environment
pub fn get_time_locale() -> &'static (Locale, UEncoding) {
    static TIME_LOCALE: OnceLock<(Locale, UEncoding)> = OnceLock::new();

    TIME_LOCALE.get_or_init(|| get_locale_from_env("LC_TIME"))
}

/// Get the numeric locale from the environment
pub fn get_numeric_locale() -> &'static (Locale, UEncoding) {
    static NUMERIC_LOCALE: OnceLock<(Locale, UEncoding)> = OnceLock::new();

    NUMERIC_LOCALE.get_or_init(|| get_locale_from_env("LC_NUMERIC"))
}

/// Return the encoding deduced from the locale environment variable.
pub fn get_locale_encoding() -> UEncoding {
    get_collating_locale().1
}
