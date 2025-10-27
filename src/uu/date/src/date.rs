// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore strtime ; (format) DATEFILE MMDDhhmm ; (vars) datetime datetimes getres AWST ACST AEST

use clap::{Arg, ArgAction, Command};
use jiff::fmt::strtime;
use jiff::tz::{TimeZone, TimeZoneDatabase};
use jiff::{Timestamp, Zoned};
#[cfg(all(unix, not(target_os = "macos"), not(target_os = "redox")))]
use libc::clock_settime;
#[cfg(all(unix, not(target_os = "redox")))]
use libc::{CLOCK_REALTIME, clock_getres, timespec};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::sync::OnceLock;
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
fn parse_military_timezone_with_offset(s: &str) -> Option<i32> {
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

    // Calculate total hours: midnight (0) + tz_offset + additional_hours
    // Midnight in timezone X converted to UTC
    let total_hours = (0 - tz_offset + additional_hours).rem_euclid(24);

    Some(total_hours)
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
    } else if matches.get_flag(OPT_RESOLUTION) {
        Format::Resolution
    } else {
        Format::Default
    };

    let date_source = if let Some(date) = matches.get_one::<String>(OPT_DATE) {
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
            date.datetime().to_zoned(TimeZone::UTC).map_err(|e| {
                USimpleError::new(1, translate!("date-error-invalid-date", "error" => e))
            })?
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
        DateSource::Human(ref input) => {
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
                parse_date(composed)
            } else if let Some(total_hours) = military_tz_with_offset {
                // Military timezone with optional hour offset
                // Convert to UTC time: midnight + military_tz_offset + additional_hours
                let date_part =
                    strtime::format("%F", &now).unwrap_or_else(|_| String::from("1970-01-01"));
                let composed = format!("{date_part} {total_hours:02}:00:00 +00:00");
                parse_date(composed)
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
                    parse_date(composed)
                } else {
                    // Fallback on parse failure of digits
                    parse_date(input)
                }
            } else {
                parse_date(input)
            };

            let iter = std::iter::once(date);
            Box::new(iter)
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
        DateSource::FileMtime(ref path) => {
            let metadata = std::fs::metadata(path)
                .map_err_context(|| path.as_os_str().to_string_lossy().to_string())?;
            let mtime = metadata.modified()?;
            let ts = Timestamp::try_from(mtime).map_err(|e| {
                USimpleError::new(
                    1,
                    translate!("date-error-cannot-set-date", "path" => path.to_string_lossy(), "error" => e),
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
        Format::Resolution => "%s.%N",
        Format::Custom(ref fmt) => fmt,
        Format::Default => "%a %b %e %X %Z %Y",
    }
}

/// Minimal disambiguation rules for highly ambiguous timezone abbreviations.
/// Only includes cases where multiple major timezones share the same abbreviation.
/// All other abbreviations are discovered dynamically from the IANA database.
///
/// Disambiguation rationale (GNU compatible):
/// - CST: Central Standard Time (US) preferred over China/Cuba Standard Time
/// - EST: Eastern Standard Time (US) preferred over Australian Eastern Standard Time
/// - IST: India Standard Time preferred over Israel/Irish Standard Time
/// - MST: Mountain Standard Time (US) preferred over Malaysia Standard Time
/// - PST: Pacific Standard Time (US) - widely used abbreviation
/// - GMT: Alias for UTC (universal)
/// - Australian timezones: AWST, ACST, AEST (cannot be dynamically discovered)
///
/// All other timezones (JST, CET, etc.) are dynamically resolved from IANA database. // spell-checker:disable-line
static PREFERRED_TZ_MAPPINGS: &[(&str, &str)] = &[
    // Universal (no ambiguity, but commonly used)
    ("UTC", "UTC"),
    ("GMT", "UTC"),
    // Highly ambiguous US timezones (GNU compatible)
    ("PST", "America/Los_Angeles"),
    ("PDT", "America/Los_Angeles"),
    ("MST", "America/Denver"),
    ("MDT", "America/Denver"),
    ("CST", "America/Chicago"), // Ambiguous: US vs China vs Cuba
    ("CDT", "America/Chicago"),
    ("EST", "America/New_York"), // Ambiguous: US vs Australia
    ("EDT", "America/New_York"),
    // Other highly ambiguous cases
    /* spell-checker: disable */
    ("IST", "Asia/Kolkata"), // Ambiguous: India vs Israel vs Ireland
    // Australian timezones (cannot be discovered from IANA location names)
    ("AWST", "Australia/Perth"),    // Australian Western Standard Time
    ("ACST", "Australia/Adelaide"), // Australian Central Standard Time
    ("ACDT", "Australia/Adelaide"), // Australian Central Daylight Time
    ("AEST", "Australia/Sydney"),   // Australian Eastern Standard Time
    ("AEDT", "Australia/Sydney"),   // Australian Eastern Daylight Time
                                    /* spell-checker: enable */
];

/// Lazy-loaded timezone abbreviation lookup map built from IANA database.
static TZ_ABBREV_CACHE: OnceLock<HashMap<String, String>> = OnceLock::new();

/// Build timezone abbreviation lookup map from IANA database.
/// Uses preferred mappings for disambiguation, then searches all timezones.
fn build_tz_abbrev_map() -> HashMap<String, String> {
    let mut map = HashMap::new();

    // First, add preferred mappings (these take precedence)
    for (abbrev, iana) in PREFERRED_TZ_MAPPINGS {
        map.insert((*abbrev).to_string(), (*iana).to_string());
    }

    // Then, try to find additional abbreviations from IANA database
    // This gives us broader coverage while respecting disambiguation preferences
    let tzdb = TimeZoneDatabase::from_env(); // spell-checker:disable-line
    // spell-checker:disable-next-line
    for tz_name in tzdb.available() {
        let tz_str = tz_name.as_str();
        // Skip if we already have a preferred mapping for this zone
        if !map.values().any(|v| v == tz_str) {
            // For zones without preferred mappings, use last component as potential abbreviation
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
    }

    map
}

/// Get IANA timezone name for a given abbreviation.
/// Uses lazy-loaded cache with preferred mappings for disambiguation.
fn tz_abbrev_to_iana(abbrev: &str) -> Option<&str> {
    let cache = TZ_ABBREV_CACHE.get_or_init(build_tz_abbrev_map);
    cache.get(abbrev).map(|s| s.as_str())
}

/// Resolve timezone abbreviation in date string and replace with numeric offset.
/// Returns the modified string with offset, or original if no abbreviation found.
fn resolve_tz_abbreviation<S: AsRef<str>>(date_str: S) -> String {
    let s = date_str.as_ref();

    // Look for timezone abbreviation at the end of the string
    // Pattern: ends with uppercase letters (2-5 chars)
    if let Some(last_word) = s.split_whitespace().last() {
        // Check if it's a potential timezone abbreviation (all uppercase, 2-5 chars)
        if last_word.len() >= 2
            && last_word.len() <= 5
            && last_word.chars().all(|c| c.is_ascii_uppercase())
        {
            if let Some(iana_name) = tz_abbrev_to_iana(last_word) {
                // Try to get the timezone
                if let Ok(tz) = TimeZone::get(iana_name) {
                    // Parse the date part (everything before the TZ abbreviation)
                    let date_part = s.trim_end_matches(last_word).trim();

                    // Try to parse the date with UTC first to get timestamp
                    let date_with_utc = format!("{date_part} +00:00");
                    if let Ok(parsed) = parse_datetime::parse_datetime(&date_with_utc) {
                        // Get timestamp from parsed date (which is already a Zoned)
                        let ts = parsed.timestamp();

                        // Get the offset for this specific timestamp in the target timezone
                        let zoned = ts.to_zoned(tz);
                        let offset_str = format!("{}", zoned.offset());

                        // Replace abbreviation with offset
                        return format!("{date_part} {offset_str}");
                    }
                }
            }
        }
    }

    // No abbreviation found or couldn't resolve, return original
    s.to_string()
}

/// Parse a `String` into a `DateTime`.
/// If it fails, return a tuple of the `String` along with its `ParseError`.
///
/// **Update for parse_datetime 0.13:**
/// - parse_datetime 0.11: returned `chrono::DateTime` → required conversion to `jiff::Zoned`
/// - parse_datetime 0.13: returns `jiff::Zoned` directly → no conversion needed
///
/// This change was necessary to fix issue #8754 (parsing large second values like
/// "12345.123456789 seconds ago" which failed in 0.11 but works in 0.13).
fn parse_date<S: AsRef<str> + Clone>(
    s: S,
) -> Result<Zoned, (String, parse_datetime::ParseDateTimeError)> {
    // First, try to resolve any timezone abbreviations
    let resolved = resolve_tz_abbreviation(s.as_ref());

    match parse_datetime::parse_datetime(&resolved) {
        Ok(date) => {
            // Convert to system timezone for display
            // (parse_datetime 0.13 returns Zoned in the input's timezone)
            let timestamp = date.timestamp();
            Ok(timestamp.to_zoned(TimeZone::try_system().unwrap_or(TimeZone::UTC)))
        }
        Err(e) => Err((s.as_ref().into(), e)),
    }
}

#[cfg(not(any(unix, windows)))]
fn get_clock_resolution() -> Timestamp {
    unimplemented!("getting clock resolution not implemented (unsupported target)");
}

#[cfg(all(unix, not(target_os = "redox")))]
fn get_clock_resolution() -> Timestamp {
    let mut timespec = timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    unsafe {
        // SAFETY: the timespec struct lives for the full duration of this function call.
        //
        // The clock_getres function can only fail if the passed clock_id is not
        // a known clock. All compliant posix implementors must support
        // CLOCK_REALTIME, therefore this function call cannot fail on any
        // compliant posix implementation.
        //
        // See more here:
        // https://pubs.opengroup.org/onlinepubs/9799919799/functions/clock_getres.html
        clock_getres(CLOCK_REALTIME, &raw mut timespec);
    }
    #[allow(clippy::unnecessary_cast)] // Cast required on 32-bit platforms
    Timestamp::constant(timespec.tv_sec as i64, timespec.tv_nsec as i32)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_military_timezone_with_offset() {
        // Valid cases: letter only, letter + digit, uppercase
        assert_eq!(parse_military_timezone_with_offset("m"), Some(12)); // UTC+12 -> 12:00 UTC
        assert_eq!(parse_military_timezone_with_offset("m9"), Some(21)); // 12 + 9 = 21
        assert_eq!(parse_military_timezone_with_offset("a5"), Some(4)); // 23 + 5 = 28 % 24 = 4
        assert_eq!(parse_military_timezone_with_offset("z"), Some(0)); // UTC+0 -> 00:00 UTC
        assert_eq!(parse_military_timezone_with_offset("M9"), Some(21)); // Uppercase works

        // Invalid cases: 'j' reserved, empty, too long, starts with digit
        assert_eq!(parse_military_timezone_with_offset("j"), None); // Reserved for local time
        assert_eq!(parse_military_timezone_with_offset(""), None); // Empty
        assert_eq!(parse_military_timezone_with_offset("m999"), None); // Too long
        assert_eq!(parse_military_timezone_with_offset("9m"), None); // Starts with digit
    }
}
