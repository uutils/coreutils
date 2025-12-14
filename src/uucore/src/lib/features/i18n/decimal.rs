// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::sync::OnceLock;

use icu_decimal::provider::DecimalSymbolsV1;
use icu_locale::Locale;
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

#[cfg(test)]
mod tests {
    use icu_locale::locale;

    use super::get_decimal_separator;

    #[test]
    fn test_simple_separator() {
        assert_eq!(get_decimal_separator(locale!("en")), ".");
        assert_eq!(get_decimal_separator(locale!("fr")), ",");
    }
}
