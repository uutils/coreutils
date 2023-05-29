use chrono::{DateTime, FixedOffset, Local, NaiveDateTime, TimeZone};

/// Formats that parse input can take.
/// Taken from `touch` core util
mod format {
    pub(crate) const ISO_8601: &str = "%Y-%m-%d";
    pub(crate) const POSIX_LOCALE: &str = "%a %b %e %H:%M:%S %Y";
    pub(crate) const YYYYMMDDHHMM_DOT_SS: &str = "%Y%m%d%H%M.%S";
    pub(crate) const YYYYMMDDHHMMSS: &str = "%Y-%m-%d %H:%M:%S.%f";
    pub(crate) const YYYYMMDDHHMMS: &str = "%Y-%m-%d %H:%M:%S";
    pub(crate) const YYYY_MM_DD_HH_MM: &str = "%Y-%m-%d %H:%M";
    pub(crate) const YYYYMMDDHHMM: &str = "%Y%m%d%H%M";
    pub(crate) const YYYYMMDDHHMM_OFFSET: &str = "%Y%m%d%H%M %z";
    pub(crate) const YYYYMMDDHHMM_UTC_OFFSET: &str = "%Y%m%d%H%MUTC%z";
    pub(crate) const YYYYMMDDHHMM_ZULU_OFFSET: &str = "%Y%m%d%H%MZ%z";
    pub(crate) const YYYYMMDDHHMM_HYPHENATED_OFFSET: &str = "%Y-%m-%d %H:%M %z";
    pub(crate) const ISO_T_SEP: &str = "%Y-%m-%dT%H:%M:%S";
    pub(crate) const UTC_OFFSET: &str = "UTC%#z";
    pub(crate) const ZULU_OFFSET: &str = "Z%#z";
}

/// Parse a `String` into a `DateTime`.
/// If it fails, return a tuple of the `String` along with its `ParseError`.
///
/// The purpose of this function is to provide a basic loose DateTime parser.
pub fn parse_datetime<S: AsRef<str> + Clone>(
    s: S,
) -> Result<DateTime<FixedOffset>, (String, chrono::format::ParseError)> {
    // TODO: Replace with a proper customiseable parsing solution using `nom`, `grmtools`, or
    // similar

    // Formats with offsets don't require NaiveDateTime workaround
    for fmt in [
        format::YYYYMMDDHHMM_OFFSET,
        format::YYYYMMDDHHMM_HYPHENATED_OFFSET,
        format::YYYYMMDDHHMM_UTC_OFFSET,
        format::YYYYMMDDHHMM_ZULU_OFFSET,
    ] {
        if let Ok(parsed) = DateTime::parse_from_str(s.as_ref(), fmt) {
            return Ok(parsed);
        }
    }

    // Parse formats with no offset, assume local time
    for fmt in [
        format::ISO_T_SEP,
        format::YYYYMMDDHHMM,
        format::YYYYMMDDHHMMS,
        format::YYYYMMDDHHMMSS,
        format::YYYY_MM_DD_HH_MM,
        format::YYYYMMDDHHMM_DOT_SS,
        format::POSIX_LOCALE,
    ] {
        if let Ok(parsed) = NaiveDateTime::parse_from_str(s.as_ref(), fmt) {
            return Ok(naive_dt_to_fixed_offset(parsed));
        }
    }

    // Parse epoch seconds
    if s.as_ref().bytes().next() == Some(b'@') {
        if let Ok(parsed) = NaiveDateTime::parse_from_str(&s.as_ref()[1..], "%s") {
            return Ok(naive_dt_to_fixed_offset(parsed));
        }
    }

    let ts = s.as_ref().to_owned() + "0000";
    // Parse date only formats - assume midnight local timezone
    for fmt in [format::ISO_8601] {
        let f = fmt.to_owned() + "%H%M";
        if let Ok(parsed) = NaiveDateTime::parse_from_str(&ts, &f) {
            return Ok(naive_dt_to_fixed_offset(parsed));
        }
    }

    // Parse offsets. chrono doesn't provide any functionality to parse
    // offsets, so instead we replicate parse_date behaviour by getting
    // the current date with local, and create a date time string at midnight,
    // before trying offset suffixes
    let local = Local::now();
    let ts = format!("{}", local.format("%Y%m%d")) + "0000" + s.as_ref();
    for fmt in [format::UTC_OFFSET, format::ZULU_OFFSET] {
        let f = format::YYYYMMDDHHMM.to_owned() + fmt;
        if let Ok(parsed) = DateTime::parse_from_str(&ts, &f) {
            return Ok(parsed);
        }
    }

    // Default parse and failure
    s.as_ref().parse().map_err(|e| (s.as_ref().into(), e))
}

// Convert NaiveDateTime to DateTime<FixedOffset> by assuming the offset
// is local time
fn naive_dt_to_fixed_offset(dt: NaiveDateTime) -> DateTime<FixedOffset> {
    let now = Local::now();
    now.with_timezone(now.offset());
    now.offset().from_local_datetime(&dt).unwrap().into()
}

#[cfg(test)]
mod tests {
    static TEST_TIME: i64 = 1613371067;

    #[cfg(test)]
    mod iso_8601 {
        use std::env;

        use crate::{parse_datetime::parse_datetime, parse_datetime::tests::TEST_TIME};

        #[test]
        fn test_t_sep() {
            env::set_var("TZ", "UTC");
            let dt = "2021-02-15T06:37:47";
            let actual = parse_datetime(dt);
            assert_eq!(actual.unwrap().timestamp(), TEST_TIME);
        }

        #[test]
        fn test_space_sep() {
            env::set_var("TZ", "UTC");
            let dt = "2021-02-15 06:37:47";
            let actual = parse_datetime(dt);
            assert_eq!(actual.unwrap().timestamp(), TEST_TIME);
        }

        #[test]
        fn test_space_sep_offset() {
            env::set_var("TZ", "UTC");
            let dt = "2021-02-14 22:37:47 -0800";
            let actual = parse_datetime(dt);
            assert_eq!(actual.unwrap().timestamp(), TEST_TIME);
        }

        #[test]
        fn test_t_sep_offset() {
            env::set_var("TZ", "UTC");
            let dt = "2021-02-14T22:37:47 -0800";
            let actual = parse_datetime(dt);
            assert_eq!(actual.unwrap().timestamp(), TEST_TIME);
        }
    }

    #[cfg(test)]
    mod offsets {
        use chrono::Local;

        use crate::parse_datetime::parse_datetime;

        #[test]
        fn test_positive_offsets() {
            let offsets = vec![
                "UTC+07:00",
                "UTC+0700",
                "UTC+07",
                "Z+07:00",
                "Z+0700",
                "Z+07",
            ];

            let expected = format!("{}{}", Local::now().format("%Y%m%d"), "0000+0700");
            for offset in offsets {
                let actual = parse_datetime(offset).unwrap();
                assert_eq!(expected, format!("{}", actual.format("%Y%m%d%H%M%z")));
            }
        }

        #[test]
        fn test_partial_offset() {
            let offsets = vec!["UTC+00:15", "UTC+0015", "Z+00:15", "Z+0015"];
            let expected = format!("{}{}", Local::now().format("%Y%m%d"), "0000+0015");
            for offset in offsets {
                let actual = parse_datetime(offset).unwrap();
                assert_eq!(expected, format!("{}", actual.format("%Y%m%d%H%M%z")));
            }
        }
    }
}
