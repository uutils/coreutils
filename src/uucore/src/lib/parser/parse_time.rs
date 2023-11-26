// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (vars) NANOS numstr
//! Parsing a duration from a string.
//!
//! Use the [`from_str`] function to parse a [`Duration`] from a string.

use std::collections::HashMap;
use std::time::Duration;

use crate::display::Quotable;
use crate::parse_time::ParseError::{
    EndOfString, IntOverflow, InvalidUnit, NoUnitPresent, UnexpectedToken,
};
use crate::parse_time::ParseState::{ExpectChrono, ExpectNum};

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
    let slice = match string.get(..len - 1) {
        Some(s) => s,
        None => return Err(format!("invalid time interval {}", string.quote())),
    };
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

/// Struct to parse a string such as '+0001 day 100years + 27 HOURS'
/// and return the parsed values as a `HashMap<ChronoUnit, i64>`.
///
/// Functionality is exposed via `DateModParser::parse(haystack)`
///
/// # Example
///
/// ```
/// use std::collections::HashMap;
/// use uucore::parse_time::{ChronoUnit, DateModParser};
///
/// let map: HashMap<ChronoUnit, i64> = DateModParser::parse("+0001 day 100years + 27 HOURS").unwrap();
/// let expected = HashMap::from([
///     (ChronoUnit::Day, 1),
///     (ChronoUnit::Year, 100),
///     (ChronoUnit::Hour, 27)
/// ]);
/// assert_eq!(map, expected);
/// ```
pub struct DateModParser<'a> {
    state: ParseState,
    cursor: usize,
    haystack: &'a [u8],
}

impl<'a> DateModParser<'a> {
    pub fn parse(haystack: &'a str) -> Result<HashMap<ChronoUnit, i64>, ParseError> {
        Self {
            state: ExpectNum,
            cursor: 0,
            haystack: haystack.as_bytes(),
        }
        ._parse()
    }

    #[allow(clippy::map_entry)]
    fn _parse(&mut self) -> Result<HashMap<ChronoUnit, i64>, ParseError> {
        let mut map = HashMap::new();
        if self.haystack.is_empty() {
            return Ok(map);
        }
        let mut curr_num = 0;
        while self.cursor < self.haystack.len() {
            match self.state {
                ExpectNum => match self.parse_num() {
                    Ok(num) => {
                        curr_num = num;
                        self.state = ExpectChrono;
                    }
                    Err(EndOfString) => {
                        return Ok(map);
                    }
                    Err(e) => {
                        return Err(e);
                    }
                },
                ExpectChrono => match self.parse_unit() {
                    Ok(chrono) => {
                        if map.contains_key(&chrono) {
                            *map.get_mut(&chrono).unwrap() += curr_num;
                        } else {
                            map.insert(chrono, curr_num);
                        }
                        self.state = ExpectNum;
                    }
                    Err(err) => {
                        return Err(err);
                    }
                },
            }
        }
        Ok(map)
    }

    fn parse_num(&mut self) -> Result<i64, ParseError> {
        self.skip_whitespace();
        if self.cursor >= self.haystack.len() {
            return Err(EndOfString);
        }

        const ASCII_0: u8 = 48;
        const ASCII_9: u8 = 57;
        let bytes = &self.haystack[self.cursor..];
        if bytes[0] == b'+' || bytes[0] == b'-' || (bytes[0] >= ASCII_0 && bytes[0] <= ASCII_9) {
            let mut nums = vec![bytes[0] as char];
            let mut i = 1;
            loop {
                if i >= bytes.len() {
                    break;
                }
                if let n @ ASCII_0..=ASCII_9 = bytes[i] {
                    nums.push(n as char);
                    if bytes[i].is_ascii_whitespace() {
                        self.cursor += 1;
                        break;
                    }
                    self.cursor += 1;
                    i += 1;
                } else if bytes[i].is_ascii_whitespace() {
                    self.cursor += 1;
                    i += 1;
                } else {
                    self.cursor += 1;
                    break;
                }
            }
            let n_as_string = nums.iter().collect::<String>();
            n_as_string.parse::<i64>().map_err(|_| IntOverflow)
        } else {
            Err(UnexpectedToken)
        }
    }

    fn parse_unit(&mut self) -> Result<ChronoUnit, ParseError> {
        self.skip_whitespace();
        if self.cursor >= self.haystack.len() {
            return Err(NoUnitPresent);
        }

        let units = [
            ("days", ChronoUnit::Day),
            ("day", ChronoUnit::Day),
            ("weeks", ChronoUnit::Week),
            ("week", ChronoUnit::Week),
            ("months", ChronoUnit::Month),
            ("month", ChronoUnit::Month),
            ("years", ChronoUnit::Year),
            ("year", ChronoUnit::Year),
            ("hours", ChronoUnit::Hour),
            ("hour", ChronoUnit::Hour),
            ("minutes", ChronoUnit::Minute),
            ("minute", ChronoUnit::Minute),
            ("seconds", ChronoUnit::Second),
            ("second", ChronoUnit::Second),
        ];
        let bytes = &self.haystack[self.cursor..].to_ascii_lowercase();
        for &(unit_str, chrono_unit) in &units {
            if bytes.starts_with(unit_str.as_bytes()) {
                self.cursor += unit_str.len();
                return Ok(chrono_unit);
            }
        }
        Err(InvalidUnit)
    }

    fn skip_whitespace(&mut self) {
        while self.cursor < self.haystack.len() && self.haystack[self.cursor].is_ascii_whitespace()
        {
            self.cursor += 1;
        }
    }
}
/// Enum to represent units of time.
#[derive(Eq, Hash, PartialEq, Debug, Copy, Clone)]
pub enum ChronoUnit {
    Day,
    Week,
    Month,
    Year,
    Hour,
    Minute,
    Second,
}

#[derive(Eq, Hash, PartialEq, Debug, Copy, Clone)]
enum ParseState {
    ExpectNum,
    ExpectChrono,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ParseError {
    UnexpectedToken,
    NoUnitPresent,
    EndOfString,
    InvalidUnit,
    IntOverflow,
}

#[cfg(test)]
mod tests {

    use super::{ChronoUnit, DateModParser};
    use crate::parse_time::from_str;
    use std::collections::HashMap;
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
    fn test_error_multi_bytes_characters() {
        assert!(from_str("10€").is_err());
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

    #[test]
    fn test_parse_ok() {
        const HAYSTACK_OK_0: &str = "-10 year-10month   +000011day    +10year";
        const HAYSTACK_OK_1: &str = "-1000yeAR10MONTH-1               day           ";
        const HAYSTACK_OK_2: &str = "-000000100MONTH-1               seconds           ";
        const HAYSTACK_OK_3: &str = "+1000SECONDS-1yearS+000111HOURs                ";
        const HAYSTACK_OK_4: &str = "+1000SECONDS-1yearS000420minuTES  ";
        const HAYSTACK_OK_5: &str = "1 Month";
        const HAYSTACK_OK_6: &str = "";

        let expected0 = HashMap::from([
            (ChronoUnit::Year, 0),
            (ChronoUnit::Day, 11),
            (ChronoUnit::Month, -10),
        ]);
        let test0 = DateModParser::parse(HAYSTACK_OK_0).unwrap();
        assert_eq!(expected0, test0);

        let expected1 = HashMap::from([
            (ChronoUnit::Year, -1000),
            (ChronoUnit::Day, -1),
            (ChronoUnit::Month, 10),
        ]);
        let test1 = DateModParser::parse(HAYSTACK_OK_1).unwrap();
        assert_eq!(expected1, test1);

        let expected2 = HashMap::from([(ChronoUnit::Second, -1), (ChronoUnit::Month, -100)]);
        let test2 = DateModParser::parse(HAYSTACK_OK_2).unwrap();
        assert_eq!(expected2, test2);

        let expected3 = HashMap::from([
            (ChronoUnit::Second, 1000),
            (ChronoUnit::Year, -1),
            (ChronoUnit::Hour, 111),
        ]);
        let test3 = DateModParser::parse(HAYSTACK_OK_3).unwrap();
        assert_eq!(expected3, test3);

        let expected4 = HashMap::from([
            (ChronoUnit::Second, 1000),
            (ChronoUnit::Year, -1),
            (ChronoUnit::Minute, 420),
        ]);
        let test4 = DateModParser::parse(HAYSTACK_OK_4).unwrap();
        assert_eq!(expected4, test4);

        let expected5 = HashMap::from([(ChronoUnit::Month, 1)]);
        let test5 = DateModParser::parse(HAYSTACK_OK_5).unwrap();
        assert_eq!(expected5, test5);

        let expected5 = HashMap::from([(ChronoUnit::Month, 1)]);
        let test5 = DateModParser::parse(HAYSTACK_OK_5).unwrap();
        assert_eq!(expected5, test5);

        let expected6 = HashMap::new();
        let test6 = DateModParser::parse(HAYSTACK_OK_6).unwrap();
        assert_eq!(expected6, test6);
    }

    #[test]
    fn test_parse_err() {
        const HAYSTACK_ERR_0: &str = "-10 yearz-10month   +000011day    +10year";
        const HAYSTACK_ERR_1: &str = "-10o0yeAR10MONTH-1               day           ";
        const HAYSTACK_ERR_2: &str = "+1000SECONDS-1yearS+000111HURs                ";
        const HAYSTACK_ERR_3: &str =
            "+100000000000000000000000000000000000000000000000000000000SECONDS  ";
        const HAYSTACK_ERR_4: &str = "+100000";
        const HAYSTACK_ERR_5: &str = "years";
        const HAYSTACK_ERR_6: &str = "----";

        let test0 = DateModParser::parse(HAYSTACK_ERR_0);
        assert!(test0.is_err());

        let test1 = DateModParser::parse(HAYSTACK_ERR_1);
        assert!(test1.is_err());

        let test2 = DateModParser::parse(HAYSTACK_ERR_2);
        assert!(test2.is_err());

        let test3 = DateModParser::parse(HAYSTACK_ERR_3);
        assert!(test3.is_err());

        let test4 = DateModParser::parse(HAYSTACK_ERR_4);
        assert!(test4.is_err());

        let test5 = DateModParser::parse(HAYSTACK_ERR_5);
        assert!(test5.is_err());

        let test6 = DateModParser::parse(HAYSTACK_ERR_6);
        assert!(test6.is_err());
    }
}
