// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::sync::OnceLock;

use icu_datetime::provider::neo::{DatetimeNamesMonthGregorianV1, MonthNames};
use icu_locale::{Locale, locale};
use icu_provider::prelude::*;

use crate::i18n::get_time_locale;

fn load_month_names(loc: &Locale) -> Option<Vec<(String, u8)>> {
    let data_locale = DataLocale::from(loc.clone());
    let abbr_attr = DataMarkerAttributes::from_str_or_panic("3");
    let request = DataRequest {
        id: DataIdentifierBorrowed::for_marker_attributes_and_locale(abbr_attr, &data_locale),
        metadata: DataRequestMetadata::default(),
    };

    let response: DataResponse<DatetimeNamesMonthGregorianV1> =
        icu_datetime::provider::Baked.load(request).ok()?;

    if let MonthNames::Linear(names) = response.payload.get() {
        let mut result = Vec::new();
        for (i, name) in names.iter().take(12).enumerate() {
            let month = (i + 1) as u8;
            let upper = name.to_uppercase();
            // Some locales use trailing periods in abbreviated months (e.g., "janv." in French).
            // Store both with and without the period so we can match either format.
            let stripped = upper.trim_end_matches('.');
            if stripped != upper {
                result.push((stripped.to_string(), month));
            }
            result.push((upper, month));
        }
        return Some(result);
    }
    None
}

fn get_month_names() -> &'static Vec<(String, u8)> {
    static MONTH_NAMES: OnceLock<Vec<(String, u8)>> = OnceLock::new();
    MONTH_NAMES.get_or_init(|| {
        let loc = get_time_locale().0.clone();
        // For undefined locale (C/POSIX), ICU returns generic month names like "M01", "M02"
        // which aren't useful for matching. Skip directly to English fallback.
        let result = if loc == locale!("und") {
            None
        } else {
            load_month_names(&loc)
        };
        result
            .or_else(|| load_month_names(&locale!("en")))
            .expect("ICU should always have English month data")
    })
}

/// Parse a month name from the beginning of the input bytes.
/// Returns month number (1-12) or 0 if not recognized.
pub fn month_parse(input: &[u8]) -> u8 {
    let input = input.trim_ascii_start();

    // Convert bytes to string for comparison. For valid UTF-8, use it directly.
    // For non-UTF-8 (e.g., Latin-1 locales), treat each byte as a Unicode codepoint.
    // This handles legacy encodings like ISO-8859-1 where byte 0xE9 = 'é'.
    let input_upper = std::str::from_utf8(input).map_or_else(
        |_| {
            input
                .iter()
                .map(|&b| b as char)
                .collect::<String>()
                .to_uppercase()
        },
        |s| s.to_uppercase(),
    );

    for (name, month) in get_month_names() {
        if input_upper.starts_with(name) {
            return *month;
        }
    }
    0
}
