// This file is part of the uutils coreutils package.
//
// (c) Anthony Deschamps <anthony.j.deschamps@gmail.com>
// (c) Sylvestre Ledru <sylvestre@debian.org>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (chrono) Datelike Timelike ; (format) DATEFILE MMDDhhmm ; (vars) datetime datetimes

mod app;

#[macro_use]
extern crate uucore;

use app::*;
use chrono::{DateTime, FixedOffset, Local, Offset, Utc};
#[cfg(windows)]
use chrono::{Datelike, Timelike};
#[cfg(all(unix, not(target_os = "macos")))]
use libc::{clock_settime, timespec, CLOCK_REALTIME};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
#[cfg(windows)]
use winapi::{
    shared::minwindef::WORD,
    um::{minwinbase::SYSTEMTIME, sysinfoapi::SetSystemTime},
};

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
            HOURS | HOUR => Iso8601Format::Hours,
            MINUTES | MINUTE => Iso8601Format::Minutes,
            SECONDS | SECOND => Iso8601Format::Seconds,
            NS => Iso8601Format::Ns,
            DATE => Iso8601Format::Date,
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
            DATE => Rfc3339Format::Date,
            SECONDS | SECOND => Rfc3339Format::Seconds,
            NS => Rfc3339Format::Ns,
            // Should be caught by clap
            _ => panic!("Invalid format: {}", s),
        }
    }
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let syntax = format!(
        "{0} [OPTION]... [+FORMAT]...
 {0} [OPTION]... [MMDDhhmm[[CC]YY][.ss]]",
        NAME
    );
    let matches = get_app(executable!())
        .usage(&syntax[..])
        .get_matches_from(args);

    let format = if let Some(form) = matches.value_of(OPT_FORMAT) {
        if !form.starts_with('+') {
            eprintln!("date: invalid date ‘{}’", form);
            return 1;
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
            eprintln!("date: invalid date ‘{}’", input);
            return 1;
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
                Err((input, _err)) => {
                    println!("date: invalid date ‘{}’", input);
                }
            }
        }
    }

    0
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
fn set_system_datetime(_date: DateTime<Utc>) -> i32 {
    unimplemented!("setting date not implemented (unsupported target)");
}

#[cfg(target_os = "macos")]
fn set_system_datetime(_date: DateTime<Utc>) -> i32 {
    eprintln!("date: setting the date is not supported by macOS");
    1
}

#[cfg(all(unix, not(target_os = "macos")))]
/// System call to set date (unix).
/// See here for more:
/// https://doc.rust-lang.org/libc/i686-unknown-linux-gnu/libc/fn.clock_settime.html
/// https://linux.die.net/man/3/clock_settime
/// https://www.gnu.org/software/libc/manual/html_node/Time-Types.html
fn set_system_datetime(date: DateTime<Utc>) -> i32 {
    let timespec = timespec {
        tv_sec: date.timestamp() as _,
        tv_nsec: date.timestamp_subsec_nanos() as _,
    };

    let result = unsafe { clock_settime(CLOCK_REALTIME, &timespec) };

    if result != 0 {
        let error = std::io::Error::last_os_error();
        eprintln!("date: cannot set date: {}", error);
        error.raw_os_error().unwrap()
    } else {
        0
    }
}

#[cfg(windows)]
/// System call to set date (Windows).
/// See here for more:
/// https://docs.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-setsystemtime
/// https://docs.microsoft.com/en-us/windows/win32/api/minwinbase/ns-minwinbase-systemtime
fn set_system_datetime(date: DateTime<Utc>) -> i32 {
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
        let error = std::io::Error::last_os_error();
        eprintln!("date: cannot set date: {}", error);
        error.raw_os_error().unwrap()
    } else {
        0
    }
}
