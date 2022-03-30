// This file is part of the uutils coreutils package.
//
// (c) Anthony Deschamps <anthony.j.deschamps@gmail.com>
// (c) Sylvestre Ledru <sylvestre@debian.org>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (chrono) Datelike Timelike ; (format) DATEFILE MMDDhhmm ; (vars) datetime datetimes

use chrono::{DateTime, FixedOffset, Local, Offset, Utc};
#[cfg(windows)]
use chrono::{Datelike, Timelike};
use clap::{crate_version, Arg, Command};
#[cfg(all(unix, not(target_os = "macos"), not(target_os = "redox")))]
use libc::{clock_settime, timespec, CLOCK_REALTIME};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use uucore::display::Quotable;
#[cfg(not(any(target_os = "macos", target_os = "redox")))]
use uucore::error::FromIo;
use uucore::error::{UResult, USimpleError};
use uucore::{format_usage, show_error};
#[cfg(windows)]
use winapi::{
    shared::minwindef::WORD,
    um::{minwinbase::SYSTEMTIME, sysinfoapi::SetSystemTime},
};

// Options
const DATE: &str = "date";
const HOURS: &str = "hours";
const MINUTES: &str = "minutes";
const SECONDS: &str = "seconds";
const HOUR: &str = "hour";
const MINUTE: &str = "minute";
const SECOND: &str = "second";
const NS: &str = "ns";

const ABOUT: &str = "print or set the system date and time";
const USAGE: &str = "\
    {} [OPTION]... [+FORMAT]...
    {} [OPTION]... [MMDDhhmm[[CC]YY][.ss]]";

const OPT_DATE: &str = "date";
const OPT_FORMAT: &str = "format";
const OPT_FILE: &str = "file";
const OPT_DEBUG: &str = "debug";
const OPT_ISO_8601: &str = "iso-8601";
const OPT_RFC_EMAIL: &str = "rfc-email";
const OPT_RFC_3339: &str = "rfc-3339";
const OPT_SET: &str = "set";
const OPT_REFERENCE: &str = "reference";
const OPT_UNIVERSAL: &str = "universal";
const OPT_UNIVERSAL_2: &str = "utc";

// Help strings

static ISO_8601_HELP_STRING: &str = "output date/time in ISO 8601 format.
 FMT='date' for date only (the default),
 'hours', 'minutes', 'seconds', or 'ns'
 for date and time to the indicated precision.
 Example: 2006-08-14T02:34:56-06:00";

static RFC_5322_HELP_STRING: &str = "output date and time in RFC 5322 format.
 Example: Mon, 14 Aug 2006 02:34:56 -0600";

static RFC_3339_HELP_STRING: &str = "output date/time in RFC 3339 format.
 FMT='date', 'seconds', or 'ns'
 for date and time to the indicated precision.
 Example: 2006-08-14 02:34:56-06:00";

#[cfg(not(any(target_os = "macos", target_os = "redox")))]
static OPT_SET_HELP_STRING: &str = "set time described by STRING";
#[cfg(target_os = "macos")]
static OPT_SET_HELP_STRING: &str = "set time described by STRING (not available on mac yet)";
#[cfg(target_os = "redox")]
static OPT_SET_HELP_STRING: &str = "set time described by STRING (not available on redox yet)";

/// Settings for this program, parsed from the command line
struct Settings {
    utc: bool,
    format: Format,
    date_source: DateSource,
    set_to: Option<DateTime<FixedOffset>>,
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
}

enum Iso8601Format {
    Date,
    Hours,
    Minutes,
    Seconds,
    Ns,
}

impl<'a> From<&'a str> for Iso8601Format {
    fn from(s: &str) -> Self {
        match s {
            HOURS | HOUR => Self::Hours,
            MINUTES | MINUTE => Self::Minutes,
            SECONDS | SECOND => Self::Seconds,
            NS => Self::Ns,
            DATE => Self::Date,
            // Should be caught by clap
            _ => panic!("Invalid format: {}", s),
        }
    }
}

enum Rfc3339Format {
    Date,
    Seconds,
    Ns,
}

impl<'a> From<&'a str> for Rfc3339Format {
    fn from(s: &str) -> Self {
        match s {
            DATE => Self::Date,
            SECONDS | SECOND => Self::Seconds,
            NS => Self::Ns,
            // Should be caught by clap
            _ => panic!("Invalid format: {}", s),
        }
    }
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().get_matches_from(args);

    let format = if let Some(form) = matches.value_of(OPT_FORMAT) {
        if !form.starts_with('+') {
            return Err(USimpleError::new(
                1,
                format!("invalid date {}", form.quote()),
            ));
        }
        let form = form[1..].to_string();
        Format::Custom(form)
    } else if let Some(fmt) = matches
        .values_of(OPT_ISO_8601)
        .map(|mut iter| iter.next().unwrap_or(DATE).into())
    {
        Format::Iso8601(fmt)
    } else if matches.is_present(OPT_RFC_EMAIL) {
        Format::Rfc5322
    } else if let Some(fmt) = matches.value_of(OPT_RFC_3339).map(Into::into) {
        Format::Rfc3339(fmt)
    } else {
        Format::Default
    };

    let date_source = if let Some(date) = matches.value_of(OPT_DATE) {
        DateSource::Custom(date.into())
    } else if let Some(file) = matches.value_of(OPT_FILE) {
        DateSource::File(file.into())
    } else {
        DateSource::Now
    };

    let set_to = match matches.value_of(OPT_SET).map(parse_date) {
        None => None,
        Some(Err((input, _err))) => {
            return Err(USimpleError::new(
                1,
                format!("invalid date {}", input.quote()),
            ));
        }
        Some(Ok(date)) => Some(date),
    };

    let settings = Settings {
        utc: matches.is_present(OPT_UNIVERSAL),
        format,
        date_source,
        set_to,
    };

    if let Some(date) = settings.set_to {
        // All set time functions expect UTC datetimes.
        let date: DateTime<Utc> = if settings.utc {
            date.with_timezone(&Utc)
        } else {
            date.into()
        };

        return set_system_datetime(date);
    } else {
        // Declare a file here because it needs to outlive the `dates` iterator.
        let file: File;

        // Get the current time, either in the local time zone or UTC.
        let now: DateTime<FixedOffset> = if settings.utc {
            let now = Utc::now();
            now.with_timezone(&now.offset().fix())
        } else {
            let now = Local::now();
            now.with_timezone(now.offset())
        };

        // Iterate over all dates - whether it's a single date or a file.
        let dates: Box<dyn Iterator<Item = _>> = match settings.date_source {
            DateSource::Custom(ref input) => {
                let date = parse_date(input.clone());
                let iter = std::iter::once(date);
                Box::new(iter)
            }
            DateSource::File(ref path) => {
                file = File::open(path).unwrap();
                let lines = BufReader::new(file).lines();
                let iter = lines.filter_map(Result::ok).map(parse_date);
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
                Ok(date) => {
                    // GNU `date` uses `%N` for nano seconds, however crate::chrono uses `%f`
                    let format_string = &format_string.replace("%N", "%f");
                    let formatted = date.format(format_string).to_string().replace("%f", "%N");
                    println!("{}", formatted);
                }
                Err((input, _err)) => show_error!("invalid date {}", input.quote()),
            }
        }
    }

    Ok(())
}

pub fn uu_app<'a>() -> Command<'a> {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(OPT_DATE)
                .short('d')
                .long(OPT_DATE)
                .takes_value(true)
                .help("display time described by STRING, not 'now'"),
        )
        .arg(
            Arg::new(OPT_FILE)
                .short('f')
                .long(OPT_FILE)
                .takes_value(true)
                .help("like --date; once for each line of DATEFILE"),
        )
        .arg(
            Arg::new(OPT_ISO_8601)
                .short('I')
                .long(OPT_ISO_8601)
                .takes_value(true)
                .help(ISO_8601_HELP_STRING),
        )
        .arg(
            Arg::new(OPT_RFC_EMAIL)
                .short('R')
                .long(OPT_RFC_EMAIL)
                .help(RFC_5322_HELP_STRING),
        )
        .arg(
            Arg::new(OPT_RFC_3339)
                .long(OPT_RFC_3339)
                .takes_value(true)
                .help(RFC_3339_HELP_STRING),
        )
        .arg(
            Arg::new(OPT_DEBUG)
                .long(OPT_DEBUG)
                .help("annotate the parsed date, and warn about questionable usage to stderr"),
        )
        .arg(
            Arg::new(OPT_REFERENCE)
                .short('r')
                .long(OPT_REFERENCE)
                .takes_value(true)
                .help("display the last modification time of FILE"),
        )
        .arg(
            Arg::new(OPT_SET)
                .short('s')
                .long(OPT_SET)
                .takes_value(true)
                .help(OPT_SET_HELP_STRING),
        )
        .arg(
            Arg::new(OPT_UNIVERSAL)
                .short('u')
                .long(OPT_UNIVERSAL)
                .alias(OPT_UNIVERSAL_2)
                .help("print or set Coordinated Universal Time (UTC)"),
        )
        .arg(Arg::new(OPT_FORMAT).multiple_occurrences(false))
}

/// Return the appropriate format string for the given settings.
fn make_format_string(settings: &Settings) -> &str {
    match settings.format {
        Format::Iso8601(ref fmt) => match *fmt {
            Iso8601Format::Date => "%F",
            Iso8601Format::Hours => "%FT%H%:z",
            Iso8601Format::Minutes => "%FT%H:%M%:z",
            Iso8601Format::Seconds => "%FT%T%:z",
            Iso8601Format::Ns => "%FT%T,%f%:z",
        },
        Format::Rfc5322 => "%a, %d %h %Y %T %z",
        Format::Rfc3339(ref fmt) => match *fmt {
            Rfc3339Format::Date => "%F",
            Rfc3339Format::Seconds => "%F %T%:z",
            Rfc3339Format::Ns => "%F %T.%f%:z",
        },
        Format::Custom(ref fmt) => fmt,
        Format::Default => "%c",
    }
}

/// Parse a `String` into a `DateTime`.
/// If it fails, return a tuple of the `String` along with its `ParseError`.
fn parse_date<S: AsRef<str> + Clone>(
    s: S,
) -> Result<DateTime<FixedOffset>, (String, chrono::format::ParseError)> {
    // TODO: The GNU date command can parse a wide variety of inputs.
    s.as_ref().parse().map_err(|e| (s.as_ref().into(), e))
}

#[cfg(not(any(unix, windows)))]
fn set_system_datetime(_date: DateTime<Utc>) -> UResult<()> {
    unimplemented!("setting date not implemented (unsupported target)");
}

#[cfg(target_os = "macos")]
fn set_system_datetime(_date: DateTime<Utc>) -> UResult<()> {
    Err(USimpleError::new(
        1,
        "setting the date is not supported by macOS".to_string(),
    ))
}

#[cfg(target_os = "redox")]
fn set_system_datetime(_date: DateTime<Utc>) -> UResult<()> {
    Err(USimpleError::new(
        1,
        "setting the date is not supported by Redox".to_string(),
    ))
}

#[cfg(all(unix, not(target_os = "macos"), not(target_os = "redox")))]
/// System call to set date (unix).
/// See here for more:
/// `<https://doc.rust-lang.org/libc/i686-unknown-linux-gnu/libc/fn.clock_settime.html>`
/// `<https://linux.die.net/man/3/clock_settime>`
/// `<https://www.gnu.org/software/libc/manual/html_node/Time-Types.html>`
fn set_system_datetime(date: DateTime<Utc>) -> UResult<()> {
    let timespec = timespec {
        tv_sec: date.timestamp() as _,
        tv_nsec: date.timestamp_subsec_nanos() as _,
    };

    let result = unsafe { clock_settime(CLOCK_REALTIME, &timespec) };

    if result != 0 {
        Err(std::io::Error::last_os_error().map_err_context(|| "cannot set date".to_string()))
    } else {
        Ok(())
    }
}

#[cfg(windows)]
/// System call to set date (Windows).
/// See here for more:
/// https://docs.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-setsystemtime
/// https://docs.microsoft.com/en-us/windows/win32/api/minwinbase/ns-minwinbase-systemtime
fn set_system_datetime(date: DateTime<Utc>) -> UResult<()> {
    let system_time = SYSTEMTIME {
        wYear: date.year() as WORD,
        wMonth: date.month() as WORD,
        // Ignored
        wDayOfWeek: 0,
        wDay: date.day() as WORD,
        wHour: date.hour() as WORD,
        wMinute: date.minute() as WORD,
        wSecond: date.second() as WORD,
        // TODO: be careful of leap seconds - valid range is [0, 999] - how to handle?
        wMilliseconds: ((date.nanosecond() / 1_000_000) % 1000) as WORD,
    };

    let result = unsafe { SetSystemTime(&system_time) };

    if result == 0 {
        Err(std::io::Error::last_os_error().map_err_context(|| "cannot set date".to_string()))
    } else {
        Ok(())
    }
}
