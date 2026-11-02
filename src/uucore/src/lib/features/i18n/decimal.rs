// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::sync::OnceLock;

use icu_decimal::provider::DecimalSymbolsV1;
use icu_locale::{Locale, locale};
use icu_provider::prelude::*;

use crate::i18n::get_numeric_locale;

/// Return the decimal separator for the given locale
fn get_decimal_separator(loc: Locale) -> String {
    let data_locale = DataLocale::from(loc);

    let request = DataRequest {
        id: DataIdentifierBorrowed::for_locale(&data_locale),
        metadata: DataRequestMetadata::default(),
    };

    let response: DataResponse<DecimalSymbolsV1> =
        icu_decimal::provider::Baked.load(request).unwrap();

    response.payload.get().decimal_separator().to_string()
}

/// Return the decimal separator from the language we're working with.
/// Example:
///  Say we need to format 1000.5
///     `en_US`: 1,000.5 -> decimal separator is '.'
///     `fr_FR`: 1 000,5 -> decimal separator is ','
pub fn locale_decimal_separator() -> &'static str {
    static DECIMAL_SEP: OnceLock<String> = OnceLock::new();

    DECIMAL_SEP.get_or_init(|| get_decimal_separator(get_numeric_locale().0.clone()))
}

/// Return the grouping separator for the given locale
fn get_grouping_separator(loc: Locale) -> String {
    let data_locale = DataLocale::from(loc);

    let request = DataRequest {
        id: DataIdentifierBorrowed::for_locale(&data_locale),
        metadata: DataRequestMetadata::default(),
    };

    let response: DataResponse<DecimalSymbolsV1> =
        icu_decimal::provider::Baked.load(request).unwrap();

    response.payload.get().grouping_separator().to_string()
}

/// Return the grouping separator from the language we're working with.
/// Example:
///  Say we need to format 1,000
///     `en_US`: 1,000 -> grouping separator is ','
///     `fr_FR`: 1 000 -> grouping separator is '\u{202f}'
pub fn locale_grouping_separator() -> &'static str {
    static GROUPING_SEP: OnceLock<String> = OnceLock::new();

    GROUPING_SEP.get_or_init(|| {
        let loc = get_numeric_locale().0.clone();
        // C/POSIX locale (represented as "und") has no grouping separator.
        if loc == locale!("und") {
            String::new()
        } else {
            get_grouping_separator(loc)
        }
    })
}

/// Return the grouping separator of the `LC_NUMERIC` locale as bytes in the
/// locale's own encoding, so a Latin-1 French locale gets `0xA0` where
/// [`locale_grouping_separator`] would give a UTF-8 `U+202F`.
///
/// The C library provides it where it has `nl_langinfo_l`. Elsewhere (macOS,
/// NetBSD, OpenBSD, Windows...) the separator comes from ICU, and in a
/// non-UTF-8 locale a no-break space is written as the Latin-1 `0xA0`.
pub fn locale_grouping_separator_bytes() -> &'static [u8] {
    static GROUPING_SEP: OnceLock<Vec<u8>> = OnceLock::new();

    GROUPING_SEP.get_or_init(read_grouping_separator_bytes)
}

#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "illumos",
    target_os = "solaris"
))]
fn read_grouping_separator_bytes() -> Vec<u8> {
    // SAFETY: the locale name is a NUL-terminated literal. The locale object
    // returned by newlocale is checked for null and freed only after the
    // NUL-terminated string from nl_langinfo_l, which it owns, is copied out.
    unsafe {
        let numeric = libc::newlocale(libc::LC_NUMERIC_MASK, c"".as_ptr(), std::ptr::null_mut());
        if numeric.is_null() {
            return Vec::new();
        }
        let separator = libc::nl_langinfo_l(libc::THOUSEP, numeric);
        let bytes = if separator.is_null() {
            Vec::new()
        } else {
            std::ffi::CStr::from_ptr(separator).to_bytes().to_vec()
        };
        libc::freelocale(numeric);
        bytes
    }
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "illumos",
    target_os = "solaris"
)))]
fn read_grouping_separator_bytes() -> Vec<u8> {
    let separator = locale_grouping_separator();
    let is_no_break_space = matches!(separator, "\u{a0}" | "\u{202f}");
    if is_no_break_space && get_numeric_locale().1 != crate::i18n::UEncoding::Utf8 {
        b"\xa0".to_vec()
    } else {
        separator.as_bytes().to_vec()
    }
}

/// Insert `separator` between each group of three `digits`, counting from the right.
pub fn group_digits(digits: &[u8], separator: &[u8]) -> Vec<u8> {
    let separators = digits.len().saturating_sub(1) / 3;
    let mut grouped = Vec::with_capacity(digits.len() + separators * separator.len());
    for (index, digit) in digits.iter().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            grouped.extend_from_slice(separator);
        }
        grouped.push(*digit);
    }
    grouped
}

#[cfg(test)]
mod tests {
    use icu_locale::locale;

    use super::{get_decimal_separator, get_grouping_separator, group_digits};

    #[test]
    fn test_simple_decimal_separator() {
        assert_eq!(get_decimal_separator(locale!("en")), ".");
        assert_eq!(get_decimal_separator(locale!("fr")), ",");
    }

    #[test]
    fn test_simple_grouping_separator() {
        assert_eq!(get_grouping_separator(locale!("en")), ",");
        assert_eq!(get_grouping_separator(locale!("fr")), "\u{202f}");
    }

    #[test]
    fn test_group_digits() {
        assert_eq!(group_digits(b"", b","), b"");
        assert_eq!(group_digits(b"512", b","), b"512");
        assert_eq!(group_digits(b"4096", b","), b"4,096");
        assert_eq!(group_digits(b"7340032", b"\xa0"), b"7\xa0340\xa0032");
        assert_eq!(group_digits(b"640000", b"--"), b"640--000");
    }
}
