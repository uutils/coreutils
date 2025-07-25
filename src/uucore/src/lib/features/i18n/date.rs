// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::sync::OnceLock;

use icu_datetime::provider::neo::{
    DatetimeNamesMonthGregorianV1, MonthNames, marker_attrs::ABBR_STANDALONE,
};
use icu_locale::Locale;
use icu_provider::prelude::*;

use crate::i18n::{DEFAULT_LOCALE, get_time_locale};

#[derive(Eq, Ord, PartialEq, PartialOrd, Clone, Copy)]
/// Sortable month enum
pub enum Month {
    Unknown,
    January,
    February,
    March,
    April,
    May,
    June,
    July,
    August,
    September,
    October,
    November,
    December,
}

impl Month {
    #[inline]
    fn from_1(i: usize) -> Self {
        match i {
            1 => Self::January,
            2 => Self::February,
            3 => Self::March,
            4 => Self::April,
            5 => Self::May,
            6 => Self::June,
            7 => Self::July,
            8 => Self::August,
            9 => Self::September,
            10 => Self::October,
            11 => Self::November,
            12 => Self::December,
            _ => Self::Unknown,
        }
    }

    #[inline]
    fn from_0(i: usize) -> Self {
        Self::from_1(i.saturating_add(1))
    }
}

/// Return a vector containing all month names depending on the given locale, starting from january.
fn get_abbr_month_names(loc: Locale) -> Option<Vec<String>> {
    if loc == DEFAULT_LOCALE {
        return None;
    }

    let data_locale = DataLocale::from(loc);

    let request = DataRequest {
        id: DataIdentifierBorrowed::for_marker_attributes_and_locale(ABBR_STANDALONE, &data_locale),
        metadata: DataRequestMetadata::default(),
    };

    let response: DataResponse<DatetimeNamesMonthGregorianV1> =
        icu_datetime::provider::Baked.load(request).unwrap();

    match response.payload.get() {
        MonthNames::Linear(months) => Some(months.iter().map(ToString::to_string).collect()),
        _ => todo!("unsupported"),
    }
}

pub fn locale_abbr_month_names() -> Option<&'static [String]> {
    static DECIMAL_SEP: OnceLock<Option<Vec<String>>> = OnceLock::new();

    DECIMAL_SEP
        .get_or_init(|| get_abbr_month_names(get_time_locale().0.clone()))
        .as_deref()
}

pub fn locale_parse_abbr_month(input: &[u8]) -> Month {
    // Use a match instead of a loop to improve the locale=C case
    if let Some(months) = locale_abbr_month_names() {
        months
            .iter()
            .position(|month| input.starts_with(month.as_bytes()))
            .map_or(Month::Unknown, Month::from_0)
    } else {
        match input.get(..3).map(|x| x.to_ascii_uppercase()).as_deref() {
            Some(b"JAN") => Month::January,
            Some(b"FEB") => Month::February,
            Some(b"MAR") => Month::March,
            Some(b"APR") => Month::April,
            Some(b"MAY") => Month::May,
            Some(b"JUN") => Month::June,
            Some(b"JUL") => Month::July,
            Some(b"AUG") => Month::August,
            Some(b"SEP") => Month::September,
            Some(b"OCT") => Month::October,
            Some(b"NOV") => Month::November,
            Some(b"DEC") => Month::December,
            _ => Month::Unknown,
        }
    }
}
