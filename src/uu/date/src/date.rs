// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore strtime ; (format) DATEFILE MMDDhhmm ; (vars) datetime datetimes

use clap::{Arg, ArgAction, Command};
use jiff::fmt::strtime;
use jiff::tz::TimeZone;
use jiff::{SignedDuration, Timestamp, Zoned};
#[cfg(all(unix, not(target_os = "macos"), not(target_os = "redox")))]
use libc::{CLOCK_REALTIME, clock_settime, timespec};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use uucore::error::FromIo;
use uucore::error::{UResult, USimpleError};
use uucore::translate;
use uucore::{format_usage, show};
#[cfg(windows)]
use windows_sys::Win32::{Foundation::SYSTEMTIME, System::SystemInformation::SetSystemTime};

use uucore::parser::shortcut_value_parser::ShortcutValueParser;

// Options
const DATE: &str = "date";
const HOURS: &str = "hours";
const MINUTES: &str = "minutes";
const SECONDS: &str = "seconds";
const NS: &str = "ns";

const OPT_DATE: &str = "date";
const OPT_FORMAT: &str = "format";
const OPT_FILE: &str = "file";
const OPT_DEBUG: &str = "debug";
const OPT_ISO_8601: &str = "iso-8601";
const OPT_RFC_EMAIL: &str = "rfc-email";
const OPT_RFC_822: &str = "rfc-822";
const OPT_RFC_2822: &str = "rfc-2822";
const OPT_RFC_3339: &str = "rfc-3339";
const OPT_SET: &str = "set";
const OPT_REFERENCE: &str = "reference";
const OPT_UNIVERSAL: &str = "universal";
const OPT_UNIVERSAL_2: &str = "utc";

/// Settings for this program, parsed from the command line
struct Settings {
    utc: bool,
    format: Format,
    date_source: DateSource,
    set_to: Option<Zoned>,
}

/// Various ways of displaying the date
enum Format {
    Iso8601(Iso8601Format),
    Rfc5322,
    Rfc3339(Rfc3339Format),
    Custom(String),
    Default,
}

/// Various places that dates can come from
enum DateSource {
    Now,
    Custom(String),
    File(PathBuf),
    Stdin,
    Human(SignedDuration),
}

enum Iso8601Format {
    Date,
    Hours,
    Minutes,
    Seconds,
    Ns,
}

impl From<&str> for Iso8601Format {
    fn from(s: &str) -> Self {
        match s {
            HOURS => Self::Hours,
            MINUTES => Self::Minutes,
            SECONDS => Self::Seconds,
            NS => Self::Ns,
            DATE => Self::Date,
            // Note: This is caught by clap via `possible_values`
            _ => unreachable!(),
        }
    }
}

enum Rfc3339Format {
    Date,
    Seconds,
    Ns,
}

impl From<&str> for Rfc3339Format {
    fn from(s: &str) -> Self {
        match s {
            DATE => Self::Date,
            SECONDS => Self::Seconds,
            NS => Self::Ns,
            // Should be caught by clap
            _ => panic!("Invalid format: {s}"),
        }
    }
}

#[uucore::main]
#[allow(clippy::cognitive_complexity)]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;

    let format = if let Some(form) = matches.get_one::<String>(OPT_FORMAT) {
        if !form.starts_with('+') {
            return Err(USimpleError::new(
                1,
                translate!("date-error-invalid-date", "date" => form),
            ));
        }
        let form = form[1..].to_string();
        Format::Custom(form)
    } else if let Some(fmt) = matches
        .get_many::<String>(OPT_ISO_8601)
        .map(|mut iter| iter.next().unwrap_or(&DATE.to_string()).as_str().into())
    {
        Format::Iso8601(fmt)
    } else if matches.get_flag(OPT_RFC_EMAIL) {
        Format::Rfc5322
    } else if let Some(fmt) = matches
        .get_one::<String>(OPT_RFC_3339)
        .map(|s| s.as_str().into())
    {
        Format::Rfc3339(fmt)
    } else {
        Format::Default
    };

    let date_source = if let Some(date) = matches.get_one::<String>(OPT_DATE) {
        if let Ok(duration) = parse_offset(date.as_str()) {
            DateSource::Human(duration)
        } else {
            DateSource::Custom(date.into())
        }
    } else if let Some(file) = matches.get_one::<String>(OPT_FILE) {
        match file.as_ref() {
            "-" => DateSource::Stdin,
            _ => DateSource::File(file.into()),
        }
    } else {
        DateSource::Now
    };

    let set_to = match matches.get_one::<String>(OPT_SET).map(parse_date) {
        None => None,
        Some(Err((input, _err))) => {
            return Err(USimpleError::new(
                1,
                translate!("date-error-invalid-date", "date" => input),
            ));
        }
        Some(Ok(date)) => Some(date),
    };

    let settings = Settings {
        utc: matches.get_flag(OPT_UNIVERSAL),
        format,
        date_source,
        set_to,
    };

    if let Some(date) = settings.set_to {
        // All set time functions expect UTC datetimes.
        let date = if settings.utc {
            date.with_time_zone(TimeZone::UTC)
        } else {
            date
        };

        return set_system_datetime(date);
    }

    // Get the current time, either in the local time zone or UTC.
    let now = if settings.utc {
        Timestamp::now().to_zoned(TimeZone::UTC)
    } else {
        Zoned::now()
    };

    // Iterate over all dates - whether it's a single date or a file.
    let dates: Box<dyn Iterator<Item = _>> = match settings.date_source {
        DateSource::Custom(ref input) => {
            let date = parse_date(input);
            let iter = std::iter::once(date);
            Box::new(iter)
        }
        DateSource::Human(relative_time) => {
            // Double check the result is overflow or not of the current_time + relative_time
            // it may cause a panic of chrono::datetime::DateTime add
            match now.checked_add(relative_time) {
                Ok(date) => {
                    let iter = std::iter::once(Ok(date));
                    Box::new(iter)
                }
                Err(_) => {
                    return Err(USimpleError::new(
                        1,
                        translate!("date-error-date-overflow", "date" => relative_time),
                    ));
                }
            }
        }
        DateSource::Stdin => {
            let lines = BufReader::new(std::io::stdin()).lines();
            let iter = lines.map_while(Result::ok).map(parse_date);
            Box::new(iter)
        }
        DateSource::File(ref path) => {
            if path.is_dir() {
                return Err(USimpleError::new(
                    2,
                    translate!("date-error-expected-file-got-directory", "path" => path.to_string_lossy()),
                ));
            }
            let file = File::open(path)
                .map_err_context(|| path.as_os_str().to_string_lossy().to_string())?;
            let lines = BufReader::new(file).lines();
            let iter = lines.map_while(Result::ok).map(parse_date);
            Box::new(iter)
        }
        DateSource::Now => {
            let iter = std::iter::once(Ok(now));
            Box::new(iter)
        }
    };

    let format_string = make_format_string(&settings);

    // Format all the dates
    for date in dates {
        match date {
            // TODO: Switch to lenient formatting.
            Ok(date) => match strtime::format(format_string, &date) {
                Ok(s) => println!("{s}"),
                Err(e) => {
                    return Err(USimpleError::new(
                        1,
                        translate!("date-error-invalid-format", "format" => format_string, "error" => e),
                    ));
                }
            },
            Err((input, _err)) => show!(USimpleError::new(
                1,
                translate!("date-error-invalid-date", "date" => input)
            )),
        }
    }

    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .about(translate!("date-about"))
        .override_usage(format_usage(&translate!("date-usage")))
        .infer_long_args(true)
        .arg(
            Arg::new(OPT_DATE)
                .short('d')
                .long(OPT_DATE)
                .value_name("STRING")
                .allow_hyphen_values(true)
                .overrides_with(OPT_DATE)
                .help(translate!("date-help-date")),
        )
        .arg(
            Arg::new(OPT_FILE)
                .short('f')
                .long(OPT_FILE)
                .value_name("DATEFILE")
                .value_hint(clap::ValueHint::FilePath)
                .help(translate!("date-help-file")),
        )
        .arg(
            Arg::new(OPT_ISO_8601)
                .short('I')
                .long(OPT_ISO_8601)
                .value_name("FMT")
                .value_parser(ShortcutValueParser::new([
                    DATE, HOURS, MINUTES, SECONDS, NS,
                ]))
                .num_args(0..=1)
                .default_missing_value(OPT_DATE)
                .help(translate!("date-help-iso-8601")),
        )
        .arg(
            Arg::new(OPT_RFC_EMAIL)
                .short('R')
                .long(OPT_RFC_EMAIL)
                .alias(OPT_RFC_2822)
                .alias(OPT_RFC_822)
                .help(translate!("date-help-rfc-email"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_RFC_3339)
                .long(OPT_RFC_3339)
                .value_name("FMT")
                .value_parser(ShortcutValueParser::new([DATE, SECONDS, NS]))
                .help(translate!("date-help-rfc-3339")),
        )
        .arg(
            Arg::new(OPT_DEBUG)
                .long(OPT_DEBUG)
                .help(translate!("date-help-debug"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_REFERENCE)
                .short('r')
                .long(OPT_REFERENCE)
                .value_name("FILE")
                .value_hint(clap::ValueHint::AnyPath)
                .help(translate!("date-help-reference")),
        )
        .arg(
            Arg::new(OPT_SET)
                .short('s')
                .long(OPT_SET)
                .value_name("STRING")
                .help({
                    #[cfg(not(any(target_os = "macos", target_os = "redox")))]
                    {
                        translate!("date-help-set")
                    }
                    #[cfg(target_os = "macos")]
                    {
                        translate!("date-help-set-macos")
                    }
                    #[cfg(target_os = "redox")]
                    {
                        translate!("date-help-set-redox")
                    }
                }),
        )
        .arg(
            Arg::new(OPT_UNIVERSAL)
                .short('u')
                .long(OPT_UNIVERSAL)
                .alias(OPT_UNIVERSAL_2)
                .help(translate!("date-help-universal"))
                .action(ArgAction::SetTrue),
        )
        .arg(Arg::new(OPT_FORMAT))
}

/// Return the appropriate format string for the given settings.
fn make_format_string(settings: &Settings) -> &str {
    match settings.format {
        Format::Iso8601(ref fmt) => match *fmt {
            Iso8601Format::Date => "%F",
            Iso8601Format::Hours => "%FT%H%:z",
            Iso8601Format::Minutes => "%FT%H:%M%:z",
            Iso8601Format::Seconds => "%FT%T%:z",
            Iso8601Format::Ns => "%FT%T,%N%:z",
        },
        Format::Rfc5322 => "%a, %d %h %Y %T %z",
        Format::Rfc3339(ref fmt) => match *fmt {
            Rfc3339Format::Date => "%F",
            Rfc3339Format::Seconds => "%F %T%:z",
            Rfc3339Format::Ns => "%F %T.%N%:z",
        },
        Format::Custom(ref fmt) => fmt,
        Format::Default => "%a %b %e %X %Z %Y",
    }
}

/// Parse a `String` into a `DateTime`.
/// If it fails, return a tuple of the `String` along with its `ParseError`.
// TODO: Convert `parse_datetime` to jiff and remove wrapper from chrono to jiff structures.
fn parse_date<S: AsRef<str> + Clone>(
    s: S,
) -> Result<Zoned, (String, parse_datetime::ParseDateTimeError)> {
    let input = s.as_ref();

    // Handle TZ="timezone" date_spec syntax
    if let Some((tz_spec, date_part)) = parse_tz_syntax(input) {
        parse_date_with_timezone(&tz_spec, &date_part, input)
    } else {
        // Original parsing logic - no TZ prefix
        parse_date_without_timezone(input)
    }
}

/// Parse TZ="timezone" `date_spec` syntax used by GNU date
/// Returns (`timezone_name`, `date_part`) if the syntax is detected, None otherwise
/// Handles both quoted (TZ="UTC") and unquoted (TZ=UTC) formats
fn parse_tz_syntax(input: &str) -> Option<(String, String)> {
    let trimmed = input.trim_start();

    // Check for TZ= prefix (case insensitive)
    if trimmed.len() < 3 || !trimmed[0..3].to_ascii_lowercase().starts_with("tz=") {
        return None;
    }

    let after_equals = &trimmed[3..];

    // Handle quoted timezone: TZ="..."
    if let Some(after_quote) = after_equals.strip_prefix('"') {
        if let Some(end_quote) = after_quote.find('"') {
            let tz_name = after_quote[..end_quote].to_string();
            let remainder = after_quote[end_quote + 1..].trim_start();
            if !remainder.is_empty() {
                return Some((tz_name, remainder.to_string()));
            }
        }
    }
    // Handle unquoted timezone: TZ=UTC
    else if let Some(space_pos) = after_equals.find(char::is_whitespace) {
        let tz_name = after_equals[..space_pos].to_string();
        let remainder = after_equals[space_pos..].trim_start();
        if !remainder.is_empty() {
            return Some((tz_name, remainder.to_string()));
        }
    }

    None
}

/// Parse timezone string into a jiff `TimeZone`
/// Returns `TimeZone` if valid, Err if invalid
fn parse_timezone(tz_spec: &str) -> Result<TimeZone, Box<dyn std::error::Error>> {
    // Handle very long timezone names (potential DoS)
    if tz_spec.len() > 256 {
        return Err(translate!("timezone-name-too-long").into());
    }

    // Handle common abbreviations first for consistency
    match tz_spec {
        "GMT" | "UTC" => Ok(TimeZone::UTC),
        _ => {
            // Try to parse as IANA timezone name
            if let Ok(tz) = TimeZone::get(tz_spec) {
                return Ok(tz);
            }

            // Try parsing as fixed offset (+02:00, -05:30, etc.)
            if let Ok(offset) = parse_fixed_offset(tz_spec) {
                return Ok(TimeZone::fixed(offset));
            }

            Err(translate!("unknown-timezone", "timezone" => tz_spec).into())
        }
    }
}

/// Parse fixed offset timezone strings like +02:00, -05:30, +0530, etc.
fn parse_fixed_offset(s: &str) -> Result<jiff::tz::Offset, Box<dyn std::error::Error>> {
    if s.is_empty() {
        return Err(translate!("empty-offset").into());
    }

    let (sign, rest) = match s.chars().next() {
        Some('+') => (1, &s[1..]),
        Some('-') => (-1, &s[1..]),
        _ => return Err(translate!("missing-sign").into()),
    };

    // Handle formats: HH, HHMM, HH:MM
    let (hours, minutes) = if rest.contains(':') {
        // HH:MM format
        let parts: Vec<&str> = rest.split(':').collect();
        if parts.len() != 2 {
            return Err(translate!("invalid-offset-format").into());
        }
        (parts[0].parse::<i32>()?, parts[1].parse::<i32>()?)
    } else if rest.len() == 4 {
        // HHMM format
        let hours_str = &rest[0..2];
        let mins_str = &rest[2..4];
        (hours_str.parse::<i32>()?, mins_str.parse::<i32>()?)
    } else if rest.len() == 2 {
        // HH format (assume 00 minutes)
        (rest.parse::<i32>()?, 0)
    } else {
        return Err(translate!("invalid-offset-format").into());
    };

    if hours.abs() > 23 || minutes.abs() > 59 {
        return Err(translate!("invalid-hours-or-minutes").into());
    }

    let total_seconds = sign * (hours * 3600 + minutes * 60);
    Ok(jiff::tz::Offset::from_seconds(total_seconds)?)
}

// TODO: Convert `parse_datetime` to jiff and remove wrapper from chrono to jiff structures.
// Also, consider whether parse_datetime::parse_datetime_at_date can be renamed to something
// like parse_datetime::parse_offset, instead of doing some addition/subtraction.
fn parse_offset(date: &str) -> Result<SignedDuration, ()> {
    let ref_time = chrono::Local::now();
    if let Ok(new_time) = parse_datetime::parse_datetime_at_date(ref_time, date) {
        let duration = new_time.signed_duration_since(ref_time);
        Ok(SignedDuration::new(
            duration.num_seconds(),
            duration.subsec_nanos(),
        ))
    } else {
        Err(())
    }
}

#[cfg(not(any(unix, windows)))]
fn set_system_datetime(_date: Zoned) -> UResult<()> {
    unimplemented!("setting date not implemented (unsupported target)");
}

#[cfg(target_os = "macos")]
fn set_system_datetime(_date: Zoned) -> UResult<()> {
    Err(USimpleError::new(
        1,
        translate!("date-error-setting-date-not-supported-macos"),
    ))
}

#[cfg(target_os = "redox")]
fn set_system_datetime(_date: Zoned) -> UResult<()> {
    Err(USimpleError::new(
        1,
        translate!("date-error-setting-date-not-supported-redox"),
    ))
}

#[cfg(all(unix, not(target_os = "macos"), not(target_os = "redox")))]
/// System call to set date (unix).
/// See here for more:
/// `<https://doc.rust-lang.org/libc/i686-unknown-linux-gnu/libc/fn.clock_settime.html>`
/// `<https://linux.die.net/man/3/clock_settime>`
/// `<https://www.gnu.org/software/libc/manual/html_node/Time-Types.html>`
fn set_system_datetime(date: Zoned) -> UResult<()> {
    let ts = date.timestamp();
    let timespec = timespec {
        tv_sec: ts.as_second() as _,
        tv_nsec: ts.subsec_nanosecond() as _,
    };

    let result = unsafe { clock_settime(CLOCK_REALTIME, &raw const timespec) };

    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error()
            .map_err_context(|| translate!("date-error-cannot-set-date")))
    }
}

#[cfg(windows)]
/// System call to set date (Windows).
/// See here for more:
/// * <https://docs.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-setsystemtime>
/// * <https://docs.microsoft.com/en-us/windows/win32/api/minwinbase/ns-minwinbase-systemtime>
fn set_system_datetime(date: Zoned) -> UResult<()> {
    let system_time = SYSTEMTIME {
        wYear: date.year() as u16,
        wMonth: date.month() as u16,
        // Ignored
        wDayOfWeek: 0,
        wDay: date.day() as u16,
        wHour: date.hour() as u16,
        wMinute: date.minute() as u16,
        wSecond: date.second() as u16,
        // TODO: be careful of leap seconds - valid range is [0, 999] - how to handle?
        wMilliseconds: ((date.subsec_nanosecond() / 1_000_000) % 1000) as u16,
    };

    let result = unsafe { SetSystemTime(&raw const system_time) };

    if result == 0 {
        Err(std::io::Error::last_os_error()
            .map_err_context(|| translate!("date-error-cannot-set-date")))
    } else {
        Ok(())
    }
}

/// Parse a date string with timezone specification
fn parse_date_with_timezone(
    tz_spec: &str,
    date_part: &str,
    input: &str,
) -> Result<Zoned, (String, parse_datetime::ParseDateTimeError)> {
    // Parse the timezone first
    match parse_timezone(tz_spec) {
        Ok(timezone) => {
            // Parse the date part and interpret it in the specified timezone
            match parse_datetime::parse_datetime(date_part) {
                Ok(date) => {
                    // Since parse_datetime gives us a chrono DateTime that represents the
                    // parsed civil time, we need to extract those civil components without
                    // using chrono traits. We'll format to string and re-parse with jiff.
                    let datetime_str = date.format("%Y-%m-%d %H:%M:%S").to_string();

                    // Parse with jiff's civil datetime parser
                    match jiff::civil::DateTime::strptime("%Y-%m-%d %H:%M:%S", &datetime_str) {
                        Ok(civil_dt) => {
                            // Convert to zoned datetime in the target timezone
                            match civil_dt.to_zoned(timezone.clone()) {
                                Ok(zoned) => Ok(zoned),
                                Err(_) => {
                                    // Fallback: create timestamp and apply timezone
                                    let timestamp = Timestamp::new(
                                        date.timestamp(),
                                        date.timestamp_subsec_nanos() as i32,
                                    )
                                    .unwrap();
                                    Ok(Zoned::new(timestamp, timezone))
                                }
                            }
                        }
                        Err(_) => {
                            // Fallback: create timestamp and apply timezone
                            let timestamp = Timestamp::new(
                                date.timestamp(),
                                date.timestamp_subsec_nanos() as i32,
                            )
                            .unwrap();
                            Ok(Zoned::new(timestamp, timezone))
                        }
                    }
                }
                Err(e) => Err((input.into(), e)),
            }
        }
        Err(_) => {
            // For GNU compatibility: if TZ is not recognized in TZ="..." syntax,
            // return current time instead of an error
            Ok(Zoned::now())
        }
    }
}

/// Parse a date string without timezone specification (original behavior)
fn parse_date_without_timezone(
    input: &str,
) -> Result<Zoned, (String, parse_datetime::ParseDateTimeError)> {
    match parse_datetime::parse_datetime(input) {
        Ok(date) => {
            let timestamp =
                Timestamp::new(date.timestamp(), date.timestamp_subsec_nanos() as i32).unwrap();
            Ok(Zoned::new(timestamp, TimeZone::UTC))
        }
        Err(e) => Err((input.into(), e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tz_syntax_valid() {
        // Test basic TZ syntax parsing
        let result = parse_tz_syntax("TZ=\"UTC\" 2020-01-01");
        assert_eq!(result, Some(("UTC".to_string(), "2020-01-01".to_string())));

        // Test with different timezone
        let result = parse_tz_syntax("TZ=\"EST\" now");
        assert_eq!(result, Some(("EST".to_string(), "now".to_string())));

        // Test with long timezone name
        let long_tz = "a".repeat(100);
        let input = format!("TZ=\"{long_tz}\" tomorrow");
        let result = parse_tz_syntax(&input);
        assert_eq!(result, Some((long_tz, "tomorrow".to_string())));
    }

    #[test]
    fn test_parse_tz_syntax_invalid() {
        // Test without TZ prefix
        assert_eq!(parse_tz_syntax("\"UTC\" 2020-01-01"), None);

        // Test with unclosed quote
        assert_eq!(parse_tz_syntax("TZ=\"UTC 2020-01-01"), None);

        // Test with no date part
        assert_eq!(parse_tz_syntax("TZ=\"UTC\""), None);
        assert_eq!(parse_tz_syntax("TZ=\"UTC\" "), None);
        assert_eq!(parse_tz_syntax("TZ=UTC"), None); // No date part

        // Test empty input
        assert_eq!(parse_tz_syntax(""), None);

        // Test malformed inputs
        assert_eq!(parse_tz_syntax("TX=UTC 2020-01-01"), None); // Wrong prefix
        assert_eq!(parse_tz_syntax("TZ"), None); // Too short
    }

    #[test]
    fn test_parse_tz_syntax_edge_cases() {
        // Test with empty timezone
        let result = parse_tz_syntax("TZ=\"\" 2020-01-01");
        assert_eq!(result, Some((String::new(), "2020-01-01".to_string())));

        // Test with whitespace in timezone
        let result = parse_tz_syntax("TZ=\"US/Pacific\" 2020-01-01");
        assert_eq!(
            result,
            Some(("US/Pacific".to_string(), "2020-01-01".to_string()))
        );

        // Test with multiple spaces before date part
        let result = parse_tz_syntax("TZ=\"UTC\"   tomorrow");
        assert_eq!(result, Some(("UTC".to_string(), "tomorrow".to_string())));

        // Test with quotes in timezone name (should work until first quote)
        let result = parse_tz_syntax("TZ=\"UTC\"extra\" 2020-01-01");
        assert_eq!(
            result,
            Some(("UTC".to_string(), "extra\" 2020-01-01".to_string()))
        );
    }

    #[test]
    fn test_parse_timezone_valid() {
        // Test standard timezones
        assert!(parse_timezone("UTC").is_ok());
        assert!(parse_timezone("GMT").is_ok());

        // Test that we get UTC for both (GMT maps to UTC in our impl)
        assert_eq!(parse_timezone("UTC").unwrap(), TimeZone::UTC);
        // GMT gets converted to UTC in our parse_timezone function
        let gmt_tz = parse_timezone("GMT").unwrap();
        assert_eq!(gmt_tz, TimeZone::UTC);
    }

    #[test]
    fn test_parse_timezone_fixed_offsets() {
        // Test various fixed offset formats
        assert!(parse_timezone("+02:00").is_ok());
        assert!(parse_timezone("-05:30").is_ok());
        assert!(parse_timezone("+0530").is_ok());
        assert!(parse_timezone("-08").is_ok());
    }

    #[test]
    fn test_parse_timezone_invalid() {
        // Test invalid timezones
        assert!(parse_timezone("INVALID_TZ").is_err());
        assert!(parse_timezone("").is_err());

        // Test very long timezone names
        let long_tz = "a".repeat(300);
        assert!(parse_timezone(&long_tz).is_err());
    }

    #[test]
    fn test_parse_fixed_offset() {
        // Test valid formats
        assert!(parse_fixed_offset("+02:00").is_ok());
        assert!(parse_fixed_offset("-05:30").is_ok());
        assert!(parse_fixed_offset("+0530").is_ok());
        assert!(parse_fixed_offset("-08").is_ok());

        // Test invalid formats
        assert!(parse_fixed_offset("02:00").is_err()); // Missing sign
        assert!(parse_fixed_offset("+25:00").is_err()); // Invalid hours
        assert!(parse_fixed_offset("+02:70").is_err()); // Invalid minutes
        assert!(parse_fixed_offset("").is_err()); // Empty
    }
}
