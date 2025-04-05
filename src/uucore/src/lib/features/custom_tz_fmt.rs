// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (misc) WARST zoneinfo

use chrono::{TimeZone, Utc};
use chrono_tz::{OffsetName, Tz};
use iana_time_zone::get_timezone;

/// Get the alphabetic abbreviation of the current timezone.
///
/// For example, "UTC" or "CET" or "PDT"
//
/// We need this function even for local dates as chrono(_tz) does not provide a
/// way to convert Local to a fully specified timezone with abbreviation
/// (<https://github.com/chronotope/chrono-tz/issues/13>).
//
// TODO(#7659): This should take into account the date to be printed.
// - Timezone abbreviation depends on daylight savings.
// - We should do no special conversion for UTC dates.
// - If our custom logic fails, but chrono obtained a non-UTC local timezone
//   from the system, we should not just return UTC.
fn timezone_abbreviation() -> String {
    // We need this logic as `iana_time_zone::get_timezone` does not look
    // at TZ variable: https://github.com/strawlab/iana-time-zone/issues/118.
    let tz = match std::env::var("TZ") {
        // TODO: This is not fully exhaustive, we should understand how to handle
        // invalid TZ values and more complex POSIX-specified values:
        // https://www.gnu.org/software/libc/manual/html_node/TZ-Variable.html
        Ok(s) if s == "UTC0" || s.is_empty() => Tz::Etc__UTC,
        Ok(s) => s.parse().unwrap_or(Tz::Etc__UTC),
        _ => match get_timezone() {
            Ok(tz_str) => tz_str.parse().unwrap_or(Tz::Etc__UTC),
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
    // GNU `date` uses `%N` for nano seconds, however the `chrono` crate uses `%f`.
    fmt.replace("%N", "%f")
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

    #[test]
    fn test_timezone_abbreviation() {
        // Test if a timezone abbreviation is one of the values in ok_abbr.
        // TODO(#7659): We should modify this test to 2 fixed dates, one that falls in
        // daylight savings, and the other not. But right now the abbreviation depends
        // on the current time.
        fn test_zone(zone: &str, ok_abbr: &[&str]) {
            unsafe {
                std::env::set_var("TZ", zone);
            }
            let abbr = timezone_abbreviation();
            assert!(
                ok_abbr.contains(&abbr.as_str()),
                "Timezone {zone} abbreviation {abbr} is not contained within [{}].",
                ok_abbr.join(", ")
            )
        }

        // Test a few random timezones.
        test_zone("US/Pacific", &["PST", "PDT"]);
        test_zone("Europe/Zurich", &["CEST", "CET"]);
        test_zone("Africa/Cairo", &["EET", "EEST"]); // spell-checker:disable-line
        test_zone("Asia/Taipei", &["CST"]);
        test_zone("Australia/Sydney", &["AEDT", "AEST"]); // spell-checker:disable-line
        // Looks like Pacific/Tahiti is provided in /usr/share/zoneinfo, but not in chrono-tz (yet).
        //test_zone("Pacific/Tahiti", &["-10"]); // No abbreviation?
        test_zone("Antarctica/South_Pole", &["NZDT", "NZST"]); // spell-checker:disable-line

        // TODO: This is not fully exhaustive, we should understand how to handle
        // invalid TZ values and more complex POSIX-specified values:
        // https://www.gnu.org/software/libc/manual/html_node/TZ-Variable.html
        // Examples:
        //test_zone("WART4WARST,J1/0,J365/25", &["WART", "WARST"])
        //test_zone(":Europe/Zurich", &["CEST", "CET"]);
        //test_zone("invalid", &["invalid"]);
    }
}
