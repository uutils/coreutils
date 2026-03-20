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
///     en_US: 1,000.5 -> decimal separator is '.'
///     fr_FR: 1 000,5 -> decimal separator is ','
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
///     en_US: 1,000 -> grouping separator is ','
///     fr_FR: 1 000 -> grouping separator is '\u{202f}'
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

/// Return the grouping sizes for the given locale.
/// Returns a tuple of (primary, secondary) group sizes.
/// For example:
///   - en_US: (3, 3) -> 1,000,000 (groups of 3)
///   - hi_IN: (3, 2) -> 12,34,567 (first group 3, then groups of 2)
fn get_grouping_sizes(loc: Locale) -> (u8, u8) {
    let data_locale = DataLocale::from(loc);

    let request = DataRequest {
        id: DataIdentifierBorrowed::for_locale(&data_locale),
        metadata: DataRequestMetadata::default(),
    };

    let response: DataResponse<DecimalSymbolsV1> =
        icu_decimal::provider::Baked.load(request).unwrap();

    let sizes = response.payload.get().grouping_sizes;
    (sizes.primary, sizes.secondary)
}

/// Return the grouping sizes from the language we're working with.
/// Returns a tuple of (primary, secondary) group sizes for locale-aware number formatting.
/// For most locales this returns (3, 3), but Indian locales return (3, 2).
pub fn locale_grouping_sizes() -> &'static (u8, u8) {
    static GROUPING_SIZES: OnceLock<(u8, u8)> = OnceLock::new();

    GROUPING_SIZES.get_or_init(|| {
        let loc = get_numeric_locale().0.clone();
        // C/POSIX locale (represented as "und") has no grouping.
        if loc == locale!("und") {
            (0, 0)
        } else {
            get_grouping_sizes(loc)
        }
    })
}

#[cfg(test)]
mod tests {
    use icu_locale::locale;

    use super::{get_decimal_separator, get_grouping_separator};

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
}
