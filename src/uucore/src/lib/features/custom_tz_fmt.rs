// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use chrono::{TimeZone, Utc};
use chrono_tz::{OffsetName, Tz};
use iana_time_zone::get_timezone;

/// Get the alphabetic abbreviation of the current timezone.
///
/// For example, "UTC" or "CET" or "PDT"
fn timezone_abbreviation() -> String {
    let tz = match std::env::var("TZ") {
        // TODO Support other time zones...
        Ok(s) if s == "UTC0" || s.is_empty() => Tz::Etc__UTC,
        _ => match get_timezone() {
            Ok(tz_str) => tz_str.parse().unwrap(),
            Err(_) => Tz::Etc__UTC,
        },
    };

    let offset = tz.offset_from_utc_date(&Utc::now().date_naive());
    offset.abbreviation().unwrap_or("UTC").to_string()
}

/// Adapt the given string to be accepted by the chrono library crate.
///
/// # Arguments
///
/// fmt: the format of the string
///
/// # Return
///
/// A string that can be used as parameter of the chrono functions that use formats
pub fn custom_time_format(fmt: &str) -> String {
    // TODO - Revisit when chrono 0.5 is released. https://github.com/chronotope/chrono/issues/970
    // chrono crashes on %#z, but it's the same as %z anyway.
    // GNU `date` uses `%N` for nano seconds, however the `chrono` crate uses `%f`.
    fmt.replace("%#z", "%z")
        .replace("%N", "%f")
        .replace("%Z", timezone_abbreviation().as_ref())
}

#[cfg(test)]
mod tests {
    use super::{custom_time_format, timezone_abbreviation};

    #[test]
    fn test_custom_time_format() {
        assert_eq!(custom_time_format("%Y-%m-%d %H-%M-%S"), "%Y-%m-%d %H-%M-%S");
        assert_eq!(custom_time_format("%d-%m-%Y %H-%M-%S"), "%d-%m-%Y %H-%M-%S");
        assert_eq!(custom_time_format("%Y-%m-%d %H-%M-%S"), "%Y-%m-%d %H-%M-%S");
        assert_eq!(
            custom_time_format("%Y-%m-%d %H-%M-%S.%N"),
            "%Y-%m-%d %H-%M-%S.%f"
        );
        assert_eq!(custom_time_format("%Z"), timezone_abbreviation());
    }
}
