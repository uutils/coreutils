// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore strtime ; (format) DATEFILE MMDDhhmm ; (vars) datetime datetimes getres AWST ACST AEST foobarbaz

mod format_modifiers;
mod locale;

use clap::{Arg, ArgAction, Command};
use jiff::fmt::strtime::{self, BrokenDownTime, Config, PosixCustom};
use jiff::tz::{Offset, TimeZone, TimeZoneDatabase};
use jiff::{Timestamp, Zoned};
use std::borrow::Cow;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::path::PathBuf;
use std::sync::OnceLock;
use uucore::display::Quotable;
use uucore::error::FromIo;
use uucore::error::{UResult, USimpleError};
#[cfg(feature = "i18n-datetime")]
use uucore::i18n::datetime::{localize_format_string, should_use_icu_locale};
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
const OPT_RESOLUTION: &str = "resolution";
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
    debug: bool,
}

/// Options for parsing dates
#[derive(Clone, Copy)]
struct DebugOptions {
    /// Enable debug output
    debug: bool,
    /// Warn when midnight is used without explicit time specification
    warn_midnight: bool,
}

impl DebugOptions {
    fn new(debug: bool, warn_midnight: bool) -> Self {
        Self {
            debug,
            warn_midnight,
        }
    }
}

/// Various ways of displaying the date
enum Format {
    Iso8601(Iso8601Format),
    Rfc5322,
    Rfc3339(Rfc3339Format),
    Resolution,
    Custom(String),
    Default,
}

/// Various places that dates can come from
enum DateSource {
    Now,
    File(PathBuf),
    FileMtime(PathBuf),
    Stdin,
    Human(String),
    Resolution,
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

/// Indicates whether parsing a military timezone causes the date to remain the same, roll back to the previous day, or
/// advance to the next day.
/// This can occur when applying a military timezone with an optional hour offset crosses midnight
/// in either direction.
#[derive(PartialEq, Debug)]
enum DayDelta {
    /// The date does not change
    Same,
    /// The date rolls back to the previous day.
    Previous,
    /// The date advances to the next day.
    Next,
}

/// Escape invalid UTF-8 bytes in GNU-compatible octal notation.
///
/// Converts bytes to a string with printable ASCII characters preserved
/// and non-printable/invalid UTF-8 bytes escaped as `\NNN` octal sequences.
///
/// This matches GNU date's behavior for invalid input.
///
/// # Arguments
/// * `bytes` - The byte sequence to escape
///
/// # Returns
/// A string with invalid bytes escaped in octal notation
///
/// # Example
/// ```ignore
/// let invalid = b"\xb0";
/// assert_eq!(escape_invalid_bytes(invalid), "\\260");
/// ```
fn escape_invalid_bytes(bytes: &[u8]) -> String {
    let escaped = bytes
        .iter()
        .flat_map(|&b| {
            // Preserve printable ASCII except backslash
            if (0x20..0x7f).contains(&b) && b != b'\\' {
                vec![b]
            } else {
                // Escape as octal: \NNN
                format!("\\{b:03o}").into_bytes()
            }
        })
        .collect::<Vec<u8>>();
    String::from_utf8_lossy(&escaped).into_owned()
}

/// Strip parenthesized comments from a date string.
///
/// GNU date removes balanced parentheses and their content, treating them as comments.
/// If parentheses are unbalanced, everything from the unmatched '(' onwards is ignored.
///
/// Examples:
/// - "2026(comment)-01-05" -> "2026-01-05"
/// - "1(ignore comment to eol" -> "1"
/// - "(" -> ""
/// - "((foo)2026-01-05)" -> ""
fn strip_parenthesized_comments(input: &str) -> Cow<'_, str> {
    if !input.contains('(') {
        return Cow::Borrowed(input);
    }

    let mut result = String::with_capacity(input.len());
    let mut depth = 0;

    for c in input.chars() {
        match c {
            '(' => {
                depth += 1;
            }
            ')' if depth > 0 => {
                depth -= 1;
            }
            _ if depth == 0 => {
                result.push(c);
            }
            _ => {}
        }
    }

    Cow::Owned(result)
}

/// Parse military timezone with optional hour offset.
/// Pattern: single letter (a-z except j) optionally followed by 1-2 digits.
/// Returns Some(total_hours_in_utc) or None if pattern doesn't match.
///
/// Military timezone mappings:
/// - A-I: UTC+1 to UTC+9 (J is skipped for local time)
/// - K-M: UTC+10 to UTC+12
/// - N-Y: UTC-1 to UTC-12
/// - Z: UTC+0
///
/// The hour offset from digits is added to the base military timezone offset.
/// Examples: "m" -> 12 (noon UTC), "m9" -> 21 (9pm UTC), "a5" -> 4 (4am UTC next day)
fn parse_military_timezone_with_offset(s: &str) -> Option<(i32, DayDelta)> {
    if s.is_empty() || s.len() > 3 {
        return None;
    }

    let mut chars = s.chars();
    let letter = chars.next()?.to_ascii_lowercase();

    // Check if first character is a letter (a-z, except j which is handled separately)
    if !letter.is_ascii_lowercase() || letter == 'j' {
        return None;
    }

    // Parse optional digits (1-2 digits for hour offset)
    let additional_hours: i32 = if let Some(rest) = chars.as_str().chars().next() {
        if !rest.is_ascii_digit() {
            return None;
        }
        chars.as_str().parse().ok()?
    } else {
        0
    };

    // Map military timezone letter to UTC offset
    let tz_offset = match letter {
        'a'..='i' => (letter as i32 - 'a' as i32) + 1, // A=+1, B=+2, ..., I=+9
        'k'..='m' => (letter as i32 - 'k' as i32) + 10, // K=+10, L=+11, M=+12
        'n'..='y' => -((letter as i32 - 'n' as i32) + 1), // N=-1, O=-2, ..., Y=-12
        'z' => 0,                                      // Z=+0
        _ => return None,
    };

    let day_delta = match additional_hours - tz_offset {
        h if h < 0 => DayDelta::Previous,
        h if h >= 24 => DayDelta::Next,
        _ => DayDelta::Same,
    };

    // Calculate total hours: midnight (0) + tz_offset + additional_hours
    // Midnight in timezone X converted to UTC
    let hours_from_midnight = (0 - tz_offset + additional_hours).rem_euclid(24);

    Some((hours_from_midnight, day_delta))
}

#[uucore::main]
#[allow(clippy::cognitive_complexity)]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;

    let date_source = if let Some(date_os) = matches.get_one::<std::ffi::OsString>(OPT_DATE) {
        // Convert OsString to String, handling invalid UTF-8 with GNU-compatible error
        let date = date_os.to_str().ok_or_else(|| {
            let bytes = date_os.as_encoded_bytes();
            let escaped_str = escape_invalid_bytes(bytes);
            USimpleError::new(1, format!("invalid date '{escaped_str}'"))
        })?;
        DateSource::Human(date.into())
    } else if let Some(file) = matches.get_one::<String>(OPT_FILE) {
        match file.as_ref() {
            "-" => DateSource::Stdin,
            _ => DateSource::File(file.into()),
        }
    } else if let Some(file) = matches.get_one::<String>(OPT_REFERENCE) {
        DateSource::FileMtime(file.into())
    } else if matches.get_flag(OPT_RESOLUTION) {
        DateSource::Resolution
    } else {
        DateSource::Now
    };

    // Check for extra operands (multiple positional arguments)
    if let Some(formats) = matches.get_many::<String>(OPT_FORMAT) {
        let format_args: Vec<&String> = formats.collect();
        if format_args.len() > 1 {
            return Err(USimpleError::new(
                1,
                translate!("date-error-extra-operand", "operand" => format_args[1]),
            ));
        }
    }

    let format = if let Some(form) = matches.get_one::<String>(OPT_FORMAT) {
        if !form.starts_with('+') {
            // if an optional Format String was found but the user has not provided an input date
            // GNU prints an invalid date Error
            if !matches!(date_source, DateSource::Human(_)) {
                return Err(USimpleError::new(
                    1,
                    translate!("date-error-invalid-date", "date" => form),
                ));
            }
            // If the user did provide an input date with the --date flag and the Format String is
            // not starting with '+' GNU prints the missing '+' error message
            return Err(USimpleError::new(
                1,
                translate!("date-error-format-missing-plus", "arg" => form),
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
    } else if matches.get_flag(OPT_RESOLUTION) {
        Format::Resolution
    } else {
        Format::Default
    };

    let utc = matches.get_flag(OPT_UNIVERSAL);
    let debug_mode = matches.get_flag(OPT_DEBUG);

    // Get the current time, either in the local time zone or UTC.
    let now = if utc {
        Timestamp::now().to_zoned(TimeZone::UTC)
    } else {
        Zoned::now()
    };

    let set_to = match matches
        .get_one::<String>(OPT_SET)
        .map(|s| parse_date(s, &now, DebugOptions::new(debug_mode, true)))
    {
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
        utc,
        format,
        date_source,
        set_to,
        debug: debug_mode,
    };

    if let Some(date) = settings.set_to {
        return set_system_datetime(convert_for_set(date, settings.utc));
    }

    // Iterate over all dates - whether it's a single date or a file.
    let dates: Box<dyn Iterator<Item = _>> = match settings.date_source {
        DateSource::Human(ref input) => {
            // GNU compatibility (Comments in parentheses)
            let input = strip_parenthesized_comments(input);
            let input = input.trim();

            // GNU compatibility (Empty string):
            // An empty string (or whitespace-only) should be treated as midnight today.
            let is_empty_or_whitespace = input.is_empty();

            // GNU compatibility (Military timezone 'J'):
            // 'J' is reserved for local time in military timezones.
            // GNU date accepts it and treats it as midnight today (00:00:00).
            let is_military_j = input.eq_ignore_ascii_case("j");

            // GNU compatibility (Military timezone with optional hour offset):
            // Single letter (a-z except j) optionally followed by 1-2 digits.
            // Letter represents midnight in that military timezone (UTC offset).
            // Digits represent additional hours to add.
            // Examples: "m" -> noon UTC (12:00); "m9" -> 21:00 UTC; "a5" -> 04:00 UTC
            let military_tz_with_offset = parse_military_timezone_with_offset(input);

            // GNU compatibility (Pure numbers in date strings):
            // - Manual: https://www.gnu.org/software/coreutils/manual/html_node/Pure-numbers-in-date-strings.html
            // - Semantics: a pure decimal number denotes today's time-of-day (HH or HHMM).
            //   Examples: "0"/"00" => 00:00 today; "7"/"07" => 07:00 today; "0700" => 07:00 today.
            // For all other forms, fall back to the general parser.
            let is_pure_digits =
                !input.is_empty() && input.len() <= 4 && input.chars().all(|c| c.is_ascii_digit());

            let date = if is_empty_or_whitespace || is_military_j {
                // Treat empty string or 'J' as midnight today (00:00:00) in local time
                let date_part =
                    strtime::format("%F", &now).unwrap_or_else(|_| String::from("1970-01-01"));
                let offset = if settings.utc {
                    String::from("+00:00")
                } else {
                    strtime::format("%:z", &now).unwrap_or_default()
                };
                let composed = if offset.is_empty() {
                    format!("{date_part} 00:00")
                } else {
                    format!("{date_part} 00:00 {offset}")
                };
                if settings.debug {
                    eprintln!("date: warning: using midnight as starting time: 00:00:00");
                }
                parse_date(composed, &now, DebugOptions::new(settings.debug, false))
            } else if let Some((total_hours, day_delta)) = military_tz_with_offset {
                // Military timezone with optional hour offset
                // Convert to UTC time: midnight + military_tz_offset + additional_hours

                // When calculating a military timezone with an optional hour offset, midnight may
                // be crossed in either direction. `day_delta` indicates whether the date remains
                // the same, moves to the previous day, or advances to the next day.
                // Changing day can result in error, this closure will help handle these errors
                // gracefully.
                let format_date_with_epoch_fallback = |date: Result<Zoned, _>| -> String {
                    date.and_then(|d| strtime::format("%F", &d))
                        .unwrap_or_else(|_| String::from("1970-01-01"))
                };
                let date_part = match day_delta {
                    DayDelta::Same => format_date_with_epoch_fallback(Ok(now.clone())),
                    DayDelta::Next => format_date_with_epoch_fallback(now.tomorrow()),
                    DayDelta::Previous => format_date_with_epoch_fallback(now.yesterday()),
                };
                let composed = format!("{date_part} {total_hours:02}:00:00 +00:00");
                parse_date(composed, &now, DebugOptions::new(settings.debug, false))
            } else if is_pure_digits {
                // Derive HH and MM from the input
                let (hh_opt, mm_opt) = if input.len() <= 2 {
                    (input.parse::<u32>().ok(), Some(0u32))
                } else {
                    let (h, m) = input.split_at(input.len() - 2);
                    (h.parse::<u32>().ok(), m.parse::<u32>().ok())
                };

                if let (Some(hh), Some(mm)) = (hh_opt, mm_opt) {
                    // Compose a concrete datetime string for today with zone offset.
                    // Use the already-determined 'now' and settings.utc to select offset.
                    let date_part =
                        strtime::format("%F", &now).unwrap_or_else(|_| String::from("1970-01-01"));
                    // If -u, force +00:00; otherwise use the local offset of 'now'.
                    let offset = if settings.utc {
                        String::from("+00:00")
                    } else {
                        strtime::format("%:z", &now).unwrap_or_default()
                    };
                    let composed = if offset.is_empty() {
                        format!("{date_part} {hh:02}:{mm:02}")
                    } else {
                        format!("{date_part} {hh:02}:{mm:02} {offset}")
                    };
                    parse_date(composed, &now, DebugOptions::new(settings.debug, false))
                } else {
                    // Fallback on parse failure of digits
                    parse_date(input, &now, DebugOptions::new(settings.debug, true))
                }
            } else {
                parse_date(input, &now, DebugOptions::new(settings.debug, true))
            };

            let iter = std::iter::once(date);
            Box::new(iter)
        }
        DateSource::Stdin => parse_dates_from_reader(
            std::io::stdin(),
            &now,
            DebugOptions::new(settings.debug, true),
        ),
        DateSource::File(ref path) => {
            if path.is_dir() {
                return Err(USimpleError::new(
                    2,
                    translate!("date-error-expected-file-got-directory", "path" => path.quote()),
                ));
            }
            let file =
                File::open(path).map_err_context(|| path.as_os_str().maybe_quote().to_string())?;
            parse_dates_from_reader(file, &now, DebugOptions::new(settings.debug, true))
        }
        DateSource::FileMtime(ref path) => {
            let metadata = std::fs::metadata(path)
                .map_err_context(|| path.as_os_str().maybe_quote().to_string())?;
            let mtime = metadata.modified()?;
            let ts = Timestamp::try_from(mtime).map_err(|e| {
                USimpleError::new(
                    1,
                    translate!("date-error-cannot-set-date", "path" => path.quote(), "error" => e),
                )
            })?;
            let date = ts.to_zoned(TimeZone::try_system().unwrap_or(TimeZone::UTC));
            let iter = std::iter::once(Ok(date));
            Box::new(iter)
        }
        DateSource::Resolution => {
            let resolution = get_clock_resolution();
            let date = resolution.to_zoned(TimeZone::system());
            let iter = std::iter::once(Ok(date));
            Box::new(iter)
        }
        DateSource::Now => {
            let iter = std::iter::once(Ok(now));
            Box::new(iter)
        }
    };

    let format_string = make_format_string(&settings);
    let mut stdout = BufWriter::new(std::io::stdout().lock());

    // Format all the dates
    let config = Config::new().custom(PosixCustom::new()).lenient(true);
    for date in dates {
        match date {
            Ok(date) => {
                let date = if settings.utc {
                    date.with_time_zone(TimeZone::UTC)
                } else {
                    date
                };
                let skip_localization =
                    matches!(settings.format, Format::Rfc5322 | Format::Rfc3339(_));
                match format_date_with_locale_aware_months(
                    &date,
                    format_string,
                    &config,
                    skip_localization,
                ) {
                    Ok(s) => writeln!(stdout, "{s}").map_err(|e| {
                        USimpleError::new(1, translate!("date-error-write", "error" => e))
                    })?,
                    Err(e) => {
                        let _ = stdout.flush();
                        return Err(USimpleError::new(
                            1,
                            translate!("date-error-invalid-format", "format" => format_string, "error" => e),
                        ));
                    }
                }
            }
            Err((input, _err)) => {
                let _ = stdout.flush();
                show!(USimpleError::new(
                    1,
                    translate!("date-error-invalid-date", "date" => input)
                ));
            }
        }
    }

    stdout
        .flush()
        .map_err(|e| USimpleError::new(1, translate!("date-error-write", "error" => e)))?;
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
                .value_parser(clap::value_parser!(std::ffi::OsString))
                .help(translate!("date-help-date")),
        )
        .arg(
            Arg::new(OPT_FILE)
                .short('f')
                .long(OPT_FILE)
                .value_name("DATEFILE")
                .value_hint(clap::ValueHint::FilePath)
                .conflicts_with(OPT_DATE)
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
            Arg::new(OPT_RESOLUTION)
                .long(OPT_RESOLUTION)
                .conflicts_with_all([OPT_DATE, OPT_FILE])
                .overrides_with(OPT_RESOLUTION)
                .help(translate!("date-help-resolution"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_RFC_EMAIL)
                .short('R')
                .long(OPT_RFC_EMAIL)
                .alias(OPT_RFC_2822)
                .alias(OPT_RFC_822)
                .overrides_with(OPT_RFC_EMAIL)
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
                .conflicts_with_all([OPT_DATE, OPT_FILE, OPT_RESOLUTION])
                .help(translate!("date-help-reference")),
        )
        .arg(
            Arg::new(OPT_SET)
                .short('s')
                .long(OPT_SET)
                .value_name("STRING")
                .allow_hyphen_values(true)
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
                .visible_alias(OPT_UNIVERSAL_2)
                .alias("uct")
                .overrides_with(OPT_UNIVERSAL)
                .help(translate!("date-help-universal"))
                .action(ArgAction::SetTrue),
        )
        .arg(Arg::new(OPT_FORMAT).num_args(0..))
}

fn format_date_with_locale_aware_months(
    date: &Zoned,
    format_string: &str,
    config: &Config<PosixCustom>,
    skip_localization: bool,
) -> Result<String, String> {
    // First check if format string has GNU modifiers (width/flags) and format if present
    // This optimization combines detection and formatting in a single pass
    if let Some(result) =
        format_modifiers::format_with_modifiers_if_present(date, format_string, config)
    {
        return result.map_err(|e| e.to_string());
    }

    let broken_down = BrokenDownTime::from(date);

    let result = if !should_use_icu_locale() || skip_localization {
        broken_down.to_string_with_config(config, format_string)
    } else {
        let fmt = localize_format_string(format_string, date.date());
        broken_down.to_string_with_config(config, &fmt)
    };

    result.map_err(|e| e.to_string())
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
        Format::Resolution => "%s.%N",
        Format::Custom(ref fmt) => fmt,
        Format::Default => locale::get_locale_default_format(),
    }
}

/// Timezone abbreviations with known fixed UTC offsets.
/// Checked first because the abbreviation encodes the exact offset
/// (e.g., EDT always means UTC-4, even in winter when New York observes EST).
/// Offset is in seconds to support half-hour zones like IST (UTC+5:30).
/// All other timezones (JST, CET, etc.) are dynamically resolved from IANA database.
/* spell-checker: disable */
static FIXED_OFFSET_ABBREVIATIONS: &[(&str, i32)] = &[
    ("UTC", 0),
    ("GMT", 0),
    // US timezones (GNU compatible)
    ("PST", -28800), // UTC-8
    ("PDT", -25200), // UTC-7
    ("MST", -25200), // UTC-7
    ("MDT", -21600), // UTC-6
    ("CST", -21600), // UTC-6 (Ambiguous: US Central, not China/Cuba)
    ("CDT", -18000), // UTC-5
    ("EST", -18000), // UTC-5
    ("EDT", -14400), // UTC-4
    // Indian Standard Time (Ambiguous: India vs Israel vs Ireland)
    ("IST", 19800), // UTC+5:30
    // Australian timezones
    ("AWST", 28800), // UTC+8
    ("ACST", 34200), // UTC+9:30
    ("ACDT", 37800), // UTC+10:30
    ("AEST", 36000), // UTC+10
    ("AEDT", 39600), // UTC+11
    // German timezones
    ("MEZ", 3600),  // UTC+1
    ("MESZ", 7200), // UTC+2
];
/* spell-checker: enable */

/// Lazy-loaded timezone abbreviation lookup map built from IANA database.
static TZ_ABBREV_CACHE: OnceLock<HashMap<String, String>> = OnceLock::new();

/// Build timezone abbreviation lookup map from IANA database.
/// This is a fallback for abbreviations not covered by FIXED_OFFSET_ABBREVIATIONS.
fn build_tz_abbrev_map() -> HashMap<String, String> {
    let mut map = HashMap::new();

    let tzdb = TimeZoneDatabase::from_env(); // spell-checker:disable-line
    // spell-checker:disable-next-line
    for tz_name in tzdb.available() {
        let tz_str = tz_name.as_str();
        // Use last component as potential abbreviation
        // e.g., "Pacific/Fiji" could map to "FIJI"
        if let Some(last_part) = tz_str.split('/').next_back() {
            let potential_abbrev = last_part.to_uppercase();
            // Only add if it looks like an abbreviation (2-5 uppercase chars)
            if potential_abbrev.len() >= 2
                && potential_abbrev.len() <= 5
                && potential_abbrev.chars().all(|c| c.is_ascii_uppercase())
            {
                map.entry(potential_abbrev)
                    .or_insert_with(|| tz_str.to_string());
            }
        }
    }

    map
}

/// Get IANA timezone name for a given abbreviation.
/// Uses lazy-loaded cache with preferred mappings for disambiguation.
fn tz_abbrev_to_iana(abbrev: &str) -> Option<&str> {
    let cache = TZ_ABBREV_CACHE.get_or_init(build_tz_abbrev_map);
    cache.get(abbrev).map(String::as_str)
}

/// Attempts to parse a date string that contains a timezone abbreviation (e.g. "EST").
///
/// If an abbreviation is found and the date is parsable, returns `Some(Zoned)`.
/// Returns `None` if no abbreviation is detected or if parsing fails, indicating
/// that standard parsing should be attempted.
fn try_parse_with_abbreviation<S: AsRef<str>>(date_str: S, now: &Zoned) -> Option<Zoned> {
    let s = date_str.as_ref();

    // Look for timezone abbreviation at the end of the string
    // Pattern: ends with uppercase letters (2-5 chars)
    if let Some(last_word) = s.split_whitespace().last() {
        // Check if it's a potential timezone abbreviation (all uppercase, 2-5 chars)
        if last_word.len() >= 2
            && last_word.len() <= 5
            && last_word.chars().all(|c| c.is_ascii_uppercase())
        {
            let tz = if let Some(&(_, offset_secs)) = FIXED_OFFSET_ABBREVIATIONS
                .iter()
                .find(|(abbr, _)| *abbr == last_word)
            {
                Offset::from_seconds(offset_secs).ok().map(TimeZone::fixed)
            } else {
                tz_abbrev_to_iana(last_word).and_then(|name| TimeZone::get(name).ok())
            };

            if let Some(tz) = tz {
                let date_part = s.trim_end_matches(last_word).trim();
                // Parse in the target timezone so "10:30 EDT" means 10:30 in EDT
                if let Ok(parsed) = parse_datetime::parse_datetime_at_date(now.clone(), date_part) {
                    let dt = parsed.datetime();
                    if let Ok(zoned) = dt.to_zoned(tz) {
                        return Some(zoned);
                    }
                }
            }
        }
    }

    // No abbreviation found or couldn't resolve, return original
    None
}

/// Parse a `String` into a `DateTime`.
/// If it fails, return a tuple of the `String` along with its `ParseError`.
/// Helper function to parse dates from a line-based reader (stdin or file)
///
/// Takes any `Read` source, reads it line by line, and parses each line as a date.
/// Returns a boxed iterator over the parse results.
fn parse_dates_from_reader<R: Read + 'static>(
    reader: R,
    now: &Zoned,
    dbg_opts: DebugOptions,
) -> Box<dyn Iterator<Item = Result<Zoned, (String, parse_datetime::ParseDateTimeError)>> + '_> {
    let lines = BufReader::new(reader).lines();
    Box::new(
        lines
            .map_while(Result::ok)
            .map(move |s| parse_date(s, now, dbg_opts)),
    )
}

///
/// **Update for parse_datetime 0.13:**
/// - parse_datetime 0.11: returned `chrono::DateTime` → required conversion to `jiff::Zoned`
/// - parse_datetime 0.13: returns `jiff::Zoned` directly → no conversion needed
///
/// This change was necessary to fix issue #8754 (parsing large second values like
/// "12345.123456789 seconds ago" which failed in 0.11 but works in 0.13).
fn parse_date<S: AsRef<str> + Clone>(
    s: S,
    now: &Zoned,
    dbg_opts: DebugOptions,
) -> Result<Zoned, (String, parse_datetime::ParseDateTimeError)> {
    let input_str = s.as_ref();

    if dbg_opts.debug {
        eprintln!("date: input string: {input_str}");
    }

    // First, try to parse any timezone abbreviations
    if let Some(zoned) = try_parse_with_abbreviation(input_str, now) {
        if dbg_opts.debug {
            eprintln!(
                "date: parsed date part: (Y-M-D) {}",
                strtime::format("%Y-%m-%d", &zoned).unwrap_or_default()
            );
            eprintln!(
                "date: parsed time part: {}",
                strtime::format("%H:%M:%S", &zoned).unwrap_or_default()
            );
            let tz_display = zoned.time_zone().iana_name().unwrap_or("system default");
            eprintln!("date: input timezone: {tz_display}");
        }
        return Ok(zoned);
    }

    match parse_datetime::parse_datetime_at_date(now.clone(), input_str) {
        // Convert to system timezone for display
        // (parse_datetime 0.13 returns Zoned in the input's timezone)
        Ok(date) => {
            let result = date.timestamp().to_zoned(now.time_zone().clone());
            if dbg_opts.debug {
                // Show final parsed date and time
                eprintln!(
                    "date: parsed date part: (Y-M-D) {}",
                    strtime::format("%Y-%m-%d", &result).unwrap_or_default()
                );
                eprintln!(
                    "date: parsed time part: {}",
                    strtime::format("%H:%M:%S", &result).unwrap_or_default()
                );

                // Show timezone information
                eprintln!("date: input timezone: system default");

                // Check if time component was specified, if not warn about midnight usage
                // Only warn for date-only inputs (no time specified), but not for epoch formats (@N)
                // or inputs that explicitly specify a time (containing ':')
                if dbg_opts.warn_midnight && !input_str.contains(':') && !input_str.contains('@') {
                    // Input likely didn't specify a time, so midnight was assumed
                    let time_str = strtime::format("%H:%M:%S", &result).unwrap_or_default();
                    if time_str == "00:00:00" {
                        eprintln!("date: warning: using midnight as starting time: 00:00:00");
                    }
                }
            }
            Ok(result)
        }
        Err(e) => Err((input_str.into(), e)),
    }
}

#[cfg(not(any(unix, windows)))]
fn get_clock_resolution() -> Timestamp {
    unimplemented!("getting clock resolution not implemented (unsupported target)");
}

#[cfg(all(unix, not(target_os = "redox")))]
/// Returns the resolution of the system’s realtime clock.
///
/// # Panics
///
/// Panics if `clock_getres` fails. On a POSIX-compliant system this should not occur,
/// as `CLOCK_REALTIME` is required to be supported.
/// Failure would indicate a non-conforming or otherwise broken implementation.
fn get_clock_resolution() -> Timestamp {
    use nix::time::{ClockId, clock_getres};

    let timespec = clock_getres(ClockId::CLOCK_REALTIME).unwrap();

    #[allow(clippy::unnecessary_cast)] // Cast required on 32-bit platforms
    Timestamp::constant(timespec.tv_sec() as _, timespec.tv_nsec() as _)
}

#[cfg(all(unix, target_os = "redox"))]
fn get_clock_resolution() -> Timestamp {
    // Redox OS does not support the posix clock_getres function, however
    // internally it uses a resolution of 1ns to represent timestamps.
    // https://gitlab.redox-os.org/redox-os/kernel/-/blob/master/src/time.rs
    Timestamp::constant(0, 1)
}

#[cfg(windows)]
fn get_clock_resolution() -> Timestamp {
    // Windows does not expose a system call for getting the resolution of the
    // clock, however the FILETIME struct returned by GetSystemTimeAsFileTime,
    // and GetSystemTimePreciseAsFileTime has a resolution of 100ns.
    // https://learn.microsoft.com/en-us/windows/win32/api/minwinbase/ns-minwinbase-filetime
    Timestamp::constant(0, 100)
}

#[cfg(not(any(unix, windows)))]
fn set_system_datetime(_date: Zoned) -> UResult<()> {
    unimplemented!("setting date not implemented (unsupported target)");
}

/// Convert a parsed date for the system clock.
fn convert_for_set(date: Zoned, utc: bool) -> Zoned {
    if utc {
        date.timestamp().to_zoned(TimeZone::UTC)
    } else {
        date
    }
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
    use nix::{sys::time::TimeSpec, time::ClockId};

    let ts = date.timestamp();
    let timespec = TimeSpec::new(ts.as_second() as _, ts.subsec_nanosecond() as _);

    nix::time::clock_settime(ClockId::CLOCK_REALTIME, timespec)
        .map_err_context(|| translate!("date-error-cannot-set-date"))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_military_timezone_with_offset() {
        // Valid cases: letter only, letter + digit, uppercase
        assert_eq!(
            parse_military_timezone_with_offset("m"),
            Some((12, DayDelta::Previous))
        ); // UTC+12 -> 12:00 UTC
        assert_eq!(
            parse_military_timezone_with_offset("m9"),
            Some((21, DayDelta::Previous))
        ); // 12 + 9 = 21
        assert_eq!(
            parse_military_timezone_with_offset("a5"),
            Some((4, DayDelta::Same))
        ); // 23 + 5 = 28 % 24 = 4
        assert_eq!(
            parse_military_timezone_with_offset("z"),
            Some((0, DayDelta::Same))
        ); // UTC+0 -> 00:00 UTC
        assert_eq!(
            parse_military_timezone_with_offset("M9"),
            Some((21, DayDelta::Previous))
        ); // Uppercase works

        // Invalid cases: 'j' reserved, empty, too long, starts with digit
        assert_eq!(parse_military_timezone_with_offset("j"), None); // Reserved for local time
        assert_eq!(parse_military_timezone_with_offset(""), None); // Empty
        assert_eq!(parse_military_timezone_with_offset("m999"), None); // Too long
        assert_eq!(parse_military_timezone_with_offset("9m"), None); // Starts with digit
    }

    #[test]
    fn test_abbreviation_resolves_relative_date_against_now() {
        let now = "2025-03-15T20:00:00+00:00[UTC]".parse::<Zoned>().unwrap();
        let result =
            parse_date("yesterday 10:00 GMT", &now, DebugOptions::new(false, false)).unwrap();
        assert_eq!(result.date(), jiff::civil::date(2025, 3, 14));
    }

    #[test]
    fn test_utc_conversion_preserves_offset() {
        let now = Zoned::now();

        let date = parse_date(
            "Sat 20 Mar 2021 14:53:01 AWST",
            &now,
            DebugOptions::new(false, false),
        )
        .unwrap();
        let utc = convert_for_set(date, true);
        assert_eq!((utc.hour(), utc.minute(), utc.second()), (6, 53, 1)); // AWST(+08:00) -> -8h
    }

    #[test]
    fn test_strip_parenthesized_comments() {
        assert_eq!(strip_parenthesized_comments("hello"), "hello");
        assert_eq!(strip_parenthesized_comments("2026-01-05"), "2026-01-05");
        assert_eq!(strip_parenthesized_comments("("), "");
        assert_eq!(strip_parenthesized_comments("1(comment"), "1");
        assert_eq!(
            strip_parenthesized_comments("2026-01-05(this is a comment"),
            "2026-01-05"
        );
        assert_eq!(
            strip_parenthesized_comments("2026(comment)-01-05"),
            "2026-01-05"
        );
        assert_eq!(strip_parenthesized_comments("()"), "");
        assert_eq!(strip_parenthesized_comments("((foo)2026-01-05)"), "");

        // These cases test the balanced parentheses removal feature
        // which extends beyond what GNU date strictly supports
        assert_eq!(strip_parenthesized_comments("a(b)c"), "ac");
        assert_eq!(strip_parenthesized_comments("a(b)c(d)e"), "ace");
        assert_eq!(strip_parenthesized_comments("(a)(b)"), "");

        // When parentheses are unmatched, processing stops at the unmatched opening paren
        // In this case "a(b)c(d", the (b) is balanced but (d is unmatched
        // We process "a(b)c" and stop at the unmatched "(d"
        assert_eq!(strip_parenthesized_comments("a(b)c(d"), "ac");

        // Additional edge cases for nested and complex parentheses
        assert_eq!(strip_parenthesized_comments("a(b(c)d)e"), "ae"); // Nested balanced
        assert_eq!(strip_parenthesized_comments("a(b(c)d"), "a"); // Nested unbalanced
        assert_eq!(strip_parenthesized_comments("a(b)c(d)e(f"), "ace"); // Multiple groups, last unmatched
    }
}
