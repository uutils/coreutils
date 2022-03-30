// This file is part of the uutils coreutils package.
//
// (c) Alex Lyon <arcterus@mail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (vars) NANOS numstr
//! Parsing a duration from a string.
//!
//! Use the [`from_str`] function to parse a [`Duration`] from a string.

use std::time::Duration;

use crate::display::Quotable;

/// Parse a duration from a string.
///
/// The string may contain only a number, like "123" or "4.5", or it
/// may contain a number with a unit specifier, like "123s" meaning
/// one hundred twenty three seconds or "4.5d" meaning four and a half
/// days. If no unit is specified, the unit is assumed to be seconds.
///
/// The only allowed suffixes are
///
/// * "s" for seconds,
/// * "m" for minutes,
/// * "h" for hours,
/// * "d" for days.
///
/// This function uses [`Duration::saturating_mul`] to compute the
/// number of seconds, so it does not overflow. If overflow would have
/// occurred, [`Duration::MAX`] is returned instead.
///
/// # Errors
///
/// This function returns an error if the input string is empty, the
/// input is not a valid number, or the unit specifier is invalid or
/// unknown.
///
/// # Examples
///
/// ```rust
/// use std::time::Duration;
/// use uucore::parse_time::from_str;
/// assert_eq!(from_str("123"), Ok(Duration::from_secs(123)));
/// assert_eq!(from_str("2d"), Ok(Duration::from_secs(60 * 60 * 24 * 2)));
/// ```
pub fn from_str(string: &str) -> Result<Duration, String> {
    let len = string.len();
    if len == 0 {
        return Err("empty string".to_owned());
    }
    let slice = &string[..len - 1];
    let (numstr, times) = match string.chars().next_back().unwrap() {
        's' => (slice, 1),
        'm' => (slice, 60),
        'h' => (slice, 60 * 60),
        'd' => (slice, 60 * 60 * 24),
        val if !val.is_alphabetic() => (string, 1),
        _ => {
            if string == "inf" || string == "infinity" {
                ("inf", 1)
            } else {
                return Err(format!("invalid time interval {}", string.quote()));
            }
        }
    };
    let num = numstr
        .parse::<f64>()
        .map_err(|e| format!("invalid time interval {}: {}", string.quote(), e))?;

    if num < 0. {
        return Err(format!("invalid time interval {}", string.quote()));
    }

    const NANOS_PER_SEC: u32 = 1_000_000_000;
    let whole_secs = num.trunc();
    let nanos = (num.fract() * (NANOS_PER_SEC as f64)).trunc();
    let duration = Duration::new(whole_secs as u64, nanos as u32);
    Ok(duration.saturating_mul(times))
}

#[cfg(test)]
mod tests {

    use crate::parse_time::from_str;
    use std::time::Duration;

    #[test]
    fn test_no_units() {
        assert_eq!(from_str("123"), Ok(Duration::from_secs(123)));
    }

    #[test]
    fn test_units() {
        assert_eq!(from_str("2d"), Ok(Duration::from_secs(60 * 60 * 24 * 2)));
    }

    #[test]
    fn test_saturating_mul() {
        assert_eq!(from_str("9223372036854775808d"), Ok(Duration::MAX));
    }

    #[test]
    fn test_error_empty() {
        assert!(from_str("").is_err());
    }

    #[test]
    fn test_error_invalid_unit() {
        assert!(from_str("123X").is_err());
    }

    #[test]
    fn test_error_invalid_magnitude() {
        assert!(from_str("12abc3s").is_err());
    }

    #[test]
    fn test_negative() {
        assert!(from_str("-1").is_err());
    }

    /// Test that capital letters are not allowed in suffixes.
    #[test]
    fn test_no_capital_letters() {
        assert!(from_str("1S").is_err());
        assert!(from_str("1M").is_err());
        assert!(from_str("1H").is_err());
        assert!(from_str("1D").is_err());
    }
}
