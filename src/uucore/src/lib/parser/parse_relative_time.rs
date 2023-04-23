// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use regex::Regex;
#[cfg(feature = "time")]
use time::Duration;

/// Parses a relative time string and returns a `Duration` representing the
/// relative time.
///
/// # Arguments
///
/// * `s` - A string slice representing the relative time.
///
/// # Examples
///
/// ```
/// use time::Duration;
/// let duration = parse_relative_time("+3 days");
/// assert_eq!(duration, Some(Duration::days(3)));
/// ```
///
/// # Supported formats
///
/// The function supports the following formats for relative time:
///
/// * [num] [unit] (e.g., "-1 hour", "+3 days")
/// * [unit] (e.g., "hour", "day")
/// * "now" or "today"
/// * "yesterday"
/// * "tomorrow"
///
/// [num] can be a positive or negative integer.
/// [unit] can be one of the following: "fortnight", "week", "day", "hour",
/// "minute", "min", "second", "sec" and their plural forms.
///
/// # Returns
///
/// * `Some(Duration)` - If the input string can be parsed as a relative time
/// * `None` - If the input string cannot be parsed as a relative time
///
/// # Errors
///
/// This function will return `None` if the input string cannot be parsed as a
/// relative time.
#[cfg(feature = "time")]
pub fn from_str(s: &str) -> Option<Duration> {
    let time_pattern: Regex = Regex::new(
            r"(?x)
            (?P<value>[-+]?\d*)\s*
            (?P<unit>years?|months?|fortnights?|weeks?|days?|hours?|h|minutes?|mins?|m|seconds?|secs?|s|yesterday|tomorrow|now|today)
            (\s*(?P<ago>ago))?"
        )
        .unwrap();

    let mut total_duration = Duration::ZERO;
    for capture in time_pattern.captures_iter(s) {
        let value_str = capture.name("value").unwrap().as_str();
        let value = if value_str.is_empty() {
            1
        } else {
            value_str.parse::<i64>().unwrap_or(1)
        };
        let unit = capture.name("unit").unwrap().as_str();
        let is_ago = capture.name("ago").is_some();

        let duration = match unit {
            "years" | "year" => Duration::days(value * 365),
            "months" | "month" => Duration::days(value * 30),
            "fortnights" | "fortnight" => Duration::weeks(value * 2),
            "weeks" | "week" => Duration::weeks(value),
            "days" | "day" => Duration::days(value),
            "hours" | "hour" | "h" => Duration::hours(value),
            "minutes" | "minute" | "mins" | "min" | "m" => Duration::minutes(value),
            "seconds" | "second" | "secs" | "sec" | "s" => Duration::seconds(value),
            "yesterday" => Duration::days(-1),
            "tomorrow" => Duration::days(1),
            "now" | "today" => Duration::ZERO,
            _ => return None,
        };

        total_duration = total_duration.checked_add(if is_ago { -duration } else { duration })?;
    }

    if total_duration == Duration::ZERO && !time_pattern.is_match(s) {
        None
    } else {
        Some(total_duration)
    }
}

#[cfg(test)]
mod tests {

    use super::from_str;
    #[cfg(feature = "time")]
    use time::Duration;

    #[test]
    fn test_years() {
        assert_eq!(from_str("1 year"), Some(Duration::seconds(31536000)));
        assert_eq!(from_str("-2 years"), Some(Duration::seconds(-63072000)));
        assert_eq!(from_str("2 years ago"), Some(Duration::seconds(-63072000)));
        assert_eq!(from_str("year"), Some(Duration::seconds(31536000)));
    }

    #[test]
    fn test_months() {
        assert_eq!(from_str("1 month"), Some(Duration::seconds(2592000)));
        assert_eq!(from_str("2 months"), Some(Duration::seconds(5184000)));
        assert_eq!(from_str("month"), Some(Duration::seconds(2592000)));
    }

    #[test]
    fn test_fortnights() {
        assert_eq!(from_str("1 fortnight"), Some(Duration::seconds(1209600)));
        assert_eq!(from_str("3 fortnights"), Some(Duration::seconds(3628800)));
        assert_eq!(from_str("fortnight"), Some(Duration::seconds(1209600)));
    }

    #[test]
    fn test_weeks() {
        assert_eq!(from_str("1 week"), Some(Duration::seconds(604800)));
        assert_eq!(from_str("-2 weeks"), Some(Duration::seconds(-1209600)));
        assert_eq!(from_str("2 weeks ago"), Some(Duration::seconds(-1209600)));
        assert_eq!(from_str("week"), Some(Duration::seconds(604800)));
    }

    #[test]
    fn test_days() {
        assert_eq!(from_str("1 day"), Some(Duration::seconds(86400)));
        assert_eq!(from_str("2 days ago"), Some(Duration::seconds(-172800)));
        assert_eq!(from_str("-2 days"), Some(Duration::seconds(-172800)));
        assert_eq!(from_str("day"), Some(Duration::seconds(86400)));
    }

    #[test]
    fn test_hours() {
        assert_eq!(from_str("1 hour"), Some(Duration::seconds(3600)));
        assert_eq!(from_str("1 hour ago"), Some(Duration::seconds(-3600)));
        assert_eq!(from_str("-2 hours"), Some(Duration::seconds(-7200)));
        assert_eq!(from_str("hour"), Some(Duration::seconds(3600)));
    }

    #[test]
    fn test_minutes() {
        assert_eq!(from_str("1 minute"), Some(Duration::seconds(60)));
        assert_eq!(from_str("2 minutes"), Some(Duration::seconds(120)));
        assert_eq!(from_str("min"), Some(Duration::seconds(60)));
    }

    #[test]
    fn test_seconds() {
        assert_eq!(from_str("1 second"), Some(Duration::seconds(1)));
        assert_eq!(from_str("2 seconds"), Some(Duration::seconds(2)));
        assert_eq!(from_str("sec"), Some(Duration::seconds(1)));
    }

    #[test]
    fn test_relative_days() {
        assert_eq!(from_str("now"), Some(Duration::seconds(0)));
        assert_eq!(from_str("today"), Some(Duration::seconds(0)));
        assert_eq!(from_str("yesterday"), Some(Duration::seconds(-86400)));
        assert_eq!(from_str("tomorrow"), Some(Duration::seconds(86400)));
    }

    #[test]
    fn test_no_spaces() {
        assert_eq!(from_str("-1hour"), Some(Duration::hours(-1)));
        assert_eq!(from_str("+3days"), Some(Duration::days(3)));
        assert_eq!(from_str("2weeks"), Some(Duration::weeks(2)));
        assert_eq!(from_str("+4months"), Some(Duration::days(4 * 30)));
        assert_eq!(from_str("-2years"), Some(Duration::days(-2 * 365)));
        assert_eq!(from_str("15minutes"), Some(Duration::minutes(15)));
        assert_eq!(from_str("-30seconds"), Some(Duration::seconds(-30)));
        assert_eq!(from_str("30seconds ago"), Some(Duration::seconds(-30)));
    }

    #[test]
    fn test_invalid_input() {
        assert_eq!(from_str("invalid"), None);
        assert_eq!(from_str("1 invalid"), None);
    }
}
