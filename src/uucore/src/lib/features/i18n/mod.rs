use std::sync::OnceLock;

use icu_locale::{Locale, locale};

/// The encoding specified by the locale, if specified
/// Currently only supports ASCII and UTF-8 for the sake of simplicity.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum UEncoding {
    Ascii,
    Utf8,
}

const DEFAULT_LOCALE: Locale = locale!("en-US-posix");

/// Deduce the locale from the current environment
fn get_collating_locale() -> &'static (Locale, UEncoding) {
    static COLLATING_LOCALE: OnceLock<(Locale, UEncoding)> = OnceLock::new();

    COLLATING_LOCALE.get_or_init(|| {
        // Look at 3 environment variables in the following order
        //
        // 1. LC_ALL
        // 2. LC_COLLATE
        // 3. LANG
        //
        // Or fallback on Posix locale, with ASCII encoding.

        let locale_var = std::env::var("LC_ALL")
            .or_else(|_| std::env::var("LC_COLLATE"))
            .or_else(|_| std::env::var("LANG"));

        if let Ok(locale_var_str) = locale_var {
            let mut split = locale_var_str.split(&['.', '@']);

            if let Some(simple) = split.next() {
                let bcp47 = simple.replace("_", "-");
                let locale = Locale::try_from_str(&bcp47).unwrap_or(DEFAULT_LOCALE);

                // If locale parsing failed, parse the encoding part of the
                // locale. Treat the special case of the given locale being "C"
                // which becomes the default locale.
                let encoding = if (locale != DEFAULT_LOCALE || bcp47 == "C")
                    && split.next() == Some("UTF-8")
                {
                    UEncoding::Utf8
                } else {
                    UEncoding::Ascii
                };
                return (locale, encoding);
            } else {
                return (DEFAULT_LOCALE, UEncoding::Ascii);
            };
        }
        // Default POSIX locale representing LC_ALL=C
        (DEFAULT_LOCALE, UEncoding::Ascii)
    })
}

/// Return the encoding deduced from the locale environment variable.
pub fn get_locale_encoding() -> UEncoding {
    get_collating_locale().1
}
