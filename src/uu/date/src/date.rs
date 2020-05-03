#![crate_name = "uu_date"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Anthony Deschamps <anthony.j.deschamps@gmail.com>
 * (c) Sylvestre Ledru <sylvestre@debian.org>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate chrono;

extern crate clap;
#[macro_use]
extern crate uucore;

use clap::{App, Arg};

use chrono::offset::Utc;
use chrono::{DateTime, FixedOffset, Local, Offset};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

// Options
const DATE: &str = "date";
const HOURS: &str = "hours";
const MINUTES: &str = "minutes";
const SECONDS: &str = "seconds";
const HOUR: &str = "hour";
const MINUTE: &str = "minute";
const SECOND: &str = "second";
const NS: &str = "ns";

const NAME: &str = "date";
const VERSION: &str = env!("CARGO_PKG_VERSION");
const ABOUT: &str = "print or set the system date and time";

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

pub fn uumain(args: Vec<String>) -> i32 {
    let syntax = format!(
        "{0} [OPTION]... [+FORMAT]...
 {0} [OPTION]... [MMDDhhmm[[CC]YY][.ss]]",
        NAME
    );
    let matches = App::new(executable!())
        .version(VERSION)
        .about(ABOUT)
        .usage(&syntax[..])
        .arg(
            Arg::with_name(OPT_DATE)
                .short("d")
                .long(OPT_DATE)
                .takes_value(true)
                .help("display time described by STRING, not 'now'"),
        )
        .arg(
            Arg::with_name(OPT_FILE)
                .short("f")
                .long(OPT_FILE)
                .takes_value(true)
                .help("like --date; once for each line of DATEFILE"),
        )
        .arg(
            Arg::with_name(OPT_ISO_8601)
                .short("I")
                .long(OPT_ISO_8601)
                .takes_value(true)
                .help(ISO_8601_HELP_STRING),
        )
        .arg(
            Arg::with_name(OPT_RFC_EMAIL)
                .short("R")
                .long(OPT_RFC_EMAIL)
                .help(RFC_5322_HELP_STRING),
        )
        .arg(
            Arg::with_name(OPT_RFC_3339)
                .long(OPT_RFC_3339)
                .takes_value(true)
                .help(RFC_3339_HELP_STRING),
        )
        .arg(
            Arg::with_name(OPT_DEBUG)
                .long(OPT_DEBUG)
                .help("annotate the parsed date, and warn about questionable usage to stderr"),
        )
        .arg(
            Arg::with_name(OPT_REFERENCE)
                .short("r")
                .long(OPT_REFERENCE)
                .takes_value(true)
                .help("display the last modification time of FILE"),
        )
        .arg(
            Arg::with_name(OPT_SET)
                .short("s")
                .long(OPT_SET)
                .takes_value(true)
                .help("set time described by STRING"),
        )
        .arg(
            Arg::with_name(OPT_UNIVERSAL)
                .short("u")
                .long(OPT_UNIVERSAL)
                .alias(OPT_UNIVERSAL_2)
                .help("print or set Coordinated Universal Time (UTC)"),
        )
        .arg(Arg::with_name(OPT_FORMAT).multiple(true))
        .get_matches_from(&args);

    let format = if let Some(form) = matches.value_of(OPT_FORMAT) {
        let form = form[1..].into();
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

    let settings = Settings {
        utc: matches.is_present(OPT_UNIVERSAL),
        format,
        date_source,
        // TODO: Handle this option:
        set_to: None,
    };

    if let Some(_time) = settings.set_to {
        unimplemented!();
    // Probably need to use this syscall:
    // https://doc.rust-lang.org/libc/i686-unknown-linux-gnu/libc/fn.clock_settime.html
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

        /// Parse a `String` into a `DateTime`.
        /// If it fails, return a tuple of the `String` along with its `ParseError`.
        fn parse_date(
            s: String,
        ) -> Result<DateTime<FixedOffset>, (String, chrono::format::ParseError)> {
            // TODO: The GNU date command can parse a wide variety of inputs.
            s.parse().map_err(|e| (s, e))
        }

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
                    let formatted = date.format(format_string);
                    println!("{}", formatted);
                }
                Err((input, _err)) => {
                    println!("date: invalid date '{}'", input);
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
