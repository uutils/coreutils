// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

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
/// # Unsupported formats
///
/// The function currently does not support the following formats:
///
/// * "year" and "month"
/// * Times without spaces like "-1hour"
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
    // Relative time, like "-1 hour" or "+3 days".
    //
    // TODO Add support for "year" and "month".
    // TODO Add support for times without spaces like "-1hour".
    let tokens: Vec<&str> = s.split_whitespace().collect();
    match &tokens[..] {
        [num_str, "year" | "years"] => num_str
            .parse::<i64>()
            .ok()
            .map(Duration::days)
            .map(|d| d * 365),
        ["year" | "years"] => Some(Duration::days(365)),
        [num_str, "month" | "months"] => num_str
            .parse::<i64>()
            .ok()
            .map(Duration::days)
            .map(|d| d * 30),
        ["month" | "months"] => Some(Duration::days(30)),
        [num_str, "fortnight" | "fortnights"] => {
            num_str.parse::<i64>().ok().map(|n| Duration::weeks(2 * n))
        }
        ["fortnight" | "fortnights"] => Some(Duration::weeks(2)),
        [num_str, "week" | "weeks"] => num_str.parse::<i64>().ok().map(Duration::weeks),
        ["week" | "weeks"] => Some(Duration::weeks(1)),
        [num_str, "day" | "days"] => num_str.parse::<i64>().ok().map(Duration::days),
        ["day" | "days"] => Some(Duration::days(1)),
        [num_str, "hour" | "hours"] => num_str.parse::<i64>().ok().map(Duration::hours),
        ["hour" | "hours"] => Some(Duration::hours(1)),
        [num_str, "minute" | "minutes" | "min" | "mins"] => {
            num_str.parse::<i64>().ok().map(Duration::minutes)
        }
        ["minute" | "minutes" | "min" | "mins"] => Some(Duration::minutes(1)),
        [num_str, "second" | "seconds" | "sec" | "secs"] => {
            num_str.parse::<i64>().ok().map(Duration::seconds)
        }
        ["second" | "seconds" | "sec" | "secs"] => Some(Duration::seconds(1)),
        ["now" | "today"] => Some(Duration::ZERO),
        ["yesterday"] => Some(Duration::days(-1)),
        ["tomorrow"] => Some(Duration::days(1)),
        _ => None,
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
        assert_eq!(from_str("week"), Some(Duration::seconds(604800)));
    }

    #[test]
    fn test_days() {
        assert_eq!(from_str("1 day"), Some(Duration::seconds(86400)));
        assert_eq!(from_str("-2 days"), Some(Duration::seconds(-172800)));
        assert_eq!(from_str("day"), Some(Duration::seconds(86400)));
    }

    #[test]
    fn test_hours() {
        assert_eq!(from_str("1 hour"), Some(Duration::seconds(3600)));
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
    fn test_invalid_input() {
        assert_eq!(from_str("invalid"), None);
        assert_eq!(from_str("1 invalid"), None);
    }
}
