mod timezone_date {
    /// Get the alphabetic abbreviation of the current timezone.
    ///
    /// For example, "UTC" or "CET" or "PDT".
    fn timezone_abbrev() -> &str {
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

    /// Format the given time according to a custom format string.
    pub fn custom_time_format(fmt: &str, time: DateTime<Local>) -> String {
        // TODO - Revisit when chrono 0.5 is released. https://github.com/chronotope/chrono/issues/970
        // GNU `date` uses `%N` for nano seconds, however the `chrono` crate uses `%f`.
        let fmt = fmt.replace("%N", "%f").replace("%Z", timezone_abbrev());
        time.format(&fmt).to_string()
    }
}
