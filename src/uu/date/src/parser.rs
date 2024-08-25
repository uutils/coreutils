use std::str::FromStr;

use chrono::offset::TimeZone;
use chrono::{DateTime, Datelike, FixedOffset, Local, TimeDelta, Timelike};

use lazy_static::lazy_static;
use regex::{Captures, Regex};

#[derive(Debug)]
enum Token {
    Ymd(u32, u32, u32),
    Hms(u32, u32, u32),
    Ymdhms(u32, u32, u32, u32, u32, u32),
}

trait RegexUtils {
    fn unwrap_group<T>(&self, name: &str) -> T
    where
        T: FromStr<Err: std::fmt::Debug>;
}

impl RegexUtils for Captures<'_> {
    fn unwrap_group<T>(&self, name: &str) -> T
    where
        T: FromStr<Err: std::fmt::Debug>,
    {
        self.name(name).unwrap().as_str().parse::<T>().unwrap()
    }
}

impl Token {
    fn parse_ymd(token: &str) -> Option<Self> {
        lazy_static! {
            static ref ymd_regex: Regex =
                Regex::new(r"(?<year>\d{4})-(?<month>\d{2})-(?<day>\d{2})").unwrap();
        }
        ymd_regex.captures(token).map(|m| {
            let y = m.unwrap_group("year");
            let mo = m.unwrap_group("month");
            let d = m.unwrap_group("day");
            Self::Ymd(y, mo, d)
        })
    }

    fn parse_choices(token: &str, choices: &'static str) -> Option<String> {
        let regex = Regex::new(choices).unwrap();
        regex
            .captures(token)
            .map(|m| m.get(1).unwrap().as_str().to_string())
    }

    fn parse_month_name(token: &str) -> Option<i32> {
        let choices =
            Self::parse_choices(token, r"(Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dez)")?;
        let month = match choices.as_str() {
            "Jan" => 1,
            "Feb" => 2,
            "Mar" => 3,
            "Apr" => 4,
            "May" => 5,
            "Jun" => 6,
            "Jul" => 7,
            "Aug" => 8,
            "Sep" => 9,
            "Oct" => 10,
            "Nov" => 11,
            "Dez" => 12,
            _ => unreachable!(),
        };
        Some(month)
    }

    fn parse_hm(token: &str) -> Option<Self> {
        lazy_static! {
            static ref hm_regex: Regex = Regex::new(r"(?<hour>\d{2}):(?<minute>\d{2})").unwrap();
        }
        hm_regex.captures(token).map(|m| {
            let h = m.unwrap_group("hour");
            let mi = m.unwrap_group("minute");
            Self::Hms(h, mi, 0)
        })
    }

    fn parse_hms(token: &str) -> Option<Self> {
        lazy_static! {
            static ref hms_regex: Regex =
                Regex::new(r"(?<hour>\d{2}):(?<minute>\d{2}):(?<second>\d{2})").unwrap();
        }
        hms_regex
            .captures(token)
            .map(|m| {
                let h = m.unwrap_group("hour");
                let mi = m.unwrap_group("minute");
                let s = m.unwrap_group("second");
                Self::Hms(h, mi, s)
            })
            .or_else(|| Self::parse_hm(token))
    }

    fn parse_dateunit(token: &str) -> Option<String> {
        Self::parse_choices(
            token,
            r"(?<dateunit>second|minute|hour|day|week|month|year)s?",
        )
    }

    fn parse_number_i32(token: &str) -> Option<i32> {
        lazy_static! {
            static ref number_regex: Regex = Regex::new(r"\+?(\d{1,9})$").unwrap();
        }
        number_regex
            .captures(token)
            .and_then(|m| m.get(1).unwrap().as_str().parse::<i32>().ok())
    }

    // Parses date like
    // "Jul 18 06:14:49 2024 GMT" +%s"
    fn parse_with_month(input: &str, d: &DateTime<FixedOffset>) -> Option<Self> {
        let mut tokens = input.split_whitespace();
        let month = Self::parse_month_name(tokens.next()?)?;
        let day = Self::parse_number_i32(tokens.next()?)?;
        let hms = Self::parse_hms(tokens.next()?)?;
        let year = Self::parse_number_i32(tokens.next()?).unwrap_or(d.year());
        // @TODO: Parse the timezone
        if let Self::Hms(hour, minute, second) = hms {
            // Return the value
            Some(Self::Ymdhms(
                year as u32,
                month as u32,
                day as u32,
                hour,
                minute,
                second,
            ))
        } else {
            None
        }
    }

    fn parse(input: &str, mut d: DateTime<FixedOffset>) -> Result<DateTime<FixedOffset>, String> {
        // Parsing  "Jul 18 06:14:49 2024 GMT" like dates
        if let Some(Self::Ymdhms(year, mo, day, h, m, s)) = Self::parse_with_month(input, &d) {
            d = Local
                .with_ymd_and_hms(year as i32, mo, day, h, m, s)
                .unwrap()
                .into();
            return Ok(d);
        }

        let mut tokens = input.split_whitespace().peekable();
        while let Some(token) = tokens.next() {
            // Parse YMD
            if let Some(Self::Ymd(year, mo, day)) = Self::parse_ymd(token) {
                d = Local
                    .with_ymd_and_hms(year as i32, mo, day, d.hour(), d.minute(), d.second())
                    .unwrap()
                    .into();
                continue;
            }
            // Parse HMS
            else if let Some(Self::Hms(h, mi, s)) = Self::parse_hms(token) {
                d = Local
                    .with_ymd_and_hms(d.year(), d.month(), d.day(), h, mi, s)
                    .unwrap()
                    .into();
                continue;
            }
            // Parse a number
            else if let Some(number) = Self::parse_number_i32(token) {
                let number: i64 = number.into();
                // Followed by a dateunit
                let dateunit = tokens
                    .peek()
                    .and_then(|x| Self::parse_dateunit(x))
                    .unwrap_or("hour".to_string());
                match dateunit.as_str() {
                    "second" => d += TimeDelta::seconds(number),
                    "minute" => d += TimeDelta::minutes(number),
                    "hour" => d += TimeDelta::hours(number),
                    "day" => d += TimeDelta::days(number),
                    "week" => d += TimeDelta::weeks(number),
                    "month" => d += TimeDelta::days(30),
                    "year" => d += TimeDelta::days(365),
                    _ => unreachable!(),
                };
                tokens.next(); // consume the token
                continue;
            }
            // Don't know how to parse this
            else {
                return Err(format!("Error parsing date, unexpected token {token}"));
            }
        }

        Ok(d)
    }
}

// Parse fallback for dates. It tries to parse `input` and update
// `d` accordingly.
pub fn parse_fb(
    input: &str,
    d: DateTime<FixedOffset>,
) -> Result<DateTime<FixedOffset>, (&str, parse_datetime::ParseDateTimeError)> {
    Token::parse(input, d).map_err(|_| (input, parse_datetime::ParseDateTimeError::InvalidInput))
}
