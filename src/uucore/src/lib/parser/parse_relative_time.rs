// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

#[cfg(feature = "time")]
use time::Duration;

#[cfg(feature = "time")]
pub fn from_str(s: &str) -> Option<Duration> {
    // Relative time, like "-1 hour" or "+3 days".
    //
    // TODO Add support for "year" and "month".
    // TODO Add support for times without spaces like "-1hour".
    let tokens: Vec<&str> = s.split_whitespace().collect();
    match &tokens[..] {
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
