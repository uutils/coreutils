#![crate_name = "uu_date"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Anthony Deschamps <anthony.j.deschamps@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate chrono;
#[macro_use]
extern crate clap;
extern crate uucore;

use chrono::{DateTime, FixedOffset, Offset, Local};
use chrono::offset::Utc;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

// Options
const DATE: &'static str = "date";
const HOURS: &'static str = "hours";
const MINUTES: &'static str = "minutes";
const SECONDS: &'static str = "seconds";
const NS: &'static str = "ns";

// Help strings

static ISO_8601_HELP_STRING: &'static str = "output date/time in ISO 8601 format.
 FMT='date' for date only (the default),
 'hours', 'minutes', 'seconds', or 'ns'
 for date and time to the indicated precision.
 Example: 2006-08-14T02:34:56-06:00";

static RFC_2822_HELP_STRING: &'static str = "output date and time in RFC 2822 format.
 Example: Mon, 14 Aug 2006 02:34:56 -0600";

static RFC_3339_HELP_STRING: &'static str = "output date/time in RFC 3339 format.
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
    Rfc2822,
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
            HOURS => Iso8601Format::Hours,
            MINUTES => Iso8601Format::Minutes,
            SECONDS => Iso8601Format::Seconds,
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
            SECONDS => Rfc3339Format::Seconds,
            NS => Rfc3339Format::Ns,
            // Should be caught by clap
            _ => panic!("Invalid format: {}", s),
        }
    }
}

pub fn uumain(args: Vec<String>) -> i32 {

    let settings = parse_cli(args);

    if let Some(_time) = settings.set_to {
        unimplemented!();
        // Probably need to use this syscall:
        // https://doc.rust-lang.org/libc/i686-unknown-linux-gnu/libc/fn.clock_settime.html

    } else {
        // Declare a file here because it needs to outlive the `dates` iterator.
        let file: File;

        // Get the current time, either in the local time zone or UTC.
        let now: DateTime<FixedOffset> = match settings.utc {
            true => {
                let now = Utc::now();
                now.with_timezone(&now.offset().fix())
            }
            false => {
                let now = Local::now();
                now.with_timezone(now.offset())
            }
        };

        /// Parse a `String` into a `DateTime`.
        /// If it fails, return a tuple of the `String` along with its `ParseError`.
        fn parse_date(s: String)
                      -> Result<DateTime<FixedOffset>, (String, chrono::format::ParseError)> {
            // TODO: The GNU date command can parse a wide variety of inputs.
            s.parse().map_err(|e| (s, e))
        }

        // Iterate over all dates - whether it's a single date or a file.
        let dates: Box<Iterator<Item = _>> = match settings.date_source {
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


/// Handle command line arguments.
fn parse_cli(args: Vec<String>) -> Settings {
    let matches = clap_app!(
        date =>
            (@group dates =>
             (@arg date: -d --date [STRING]
              "display time described by STRING, not 'now'")
             (@arg file: -f --file [DATEFILE]
              "like --date; once for each line of DATEFILE"))

            (@group format =>
             (@arg iso_8601: -I --("iso-8601") <FMT>
              possible_value[date hours minutes seconds ns]
              #{0, 1}
              ISO_8601_HELP_STRING)
             (@arg rfc_2822: -R --("rfc-2822")
              RFC_2822_HELP_STRING)
             (@arg rfc_3339: --("rfc-3339") <FMT>
              possible_value[date seconds ns]
              RFC_3339_HELP_STRING)
             (@arg custom_format: +takes_value {
                 |s| match s.starts_with("+") {
                     true => Ok(()),
                     false => Err(String::from("Date formats must start with a '+' character"))
                 }
             }))

            (@arg debug: --debug
             "annotate the parsed date, and warn about questionable usage to stderr")
            (@arg reference: -r --reference [FILE]
             "display the last modification time of FILE")
            (@arg set: -s --set [STRING]
             "set time described by STRING")
            (@arg utc: -u --utc --universal
             "print or set Coordinated Universal Time (UTC)"))

    // TODO: Decide whether this is appropriate.
    //   The GNU date command has an explanation of all formatting options,
    //   but the `chrono` crate has a few differences (most notably, the %Z option)
    // (after_help: include_str!("usage.txt")))
        .get_matches_from(args);


    let format = if let Some(form) = matches.value_of("custom_format") {
        let form = form[1..].into();
        Format::Custom(form)
    } else if let Some(fmt) = matches.values_of("iso_8601").map(|mut iter| {
                                                                    iter.next()
                                                                        .unwrap_or(DATE)
                                                                        .into()
                                                                }) {
        Format::Iso8601(fmt)
    } else if matches.is_present("rfc_2822") {
        Format::Rfc2822
    } else if let Some(fmt) = matches.value_of("rfc_3339").map(Into::into) {
        Format::Rfc3339(fmt)
    } else {
        Format::Default
    };

    let date_source = if let Some(date) = matches.value_of("date") {
        DateSource::Custom(date.into())
    } else if let Some(file) = matches.value_of("file") {
        DateSource::File(file.into())
    } else {
        DateSource::Now
    };

    Settings {
        utc: matches.is_present("utc"),
        format: format,
        date_source: date_source,
        // TODO: Handle this option:
        set_to: None,
    }
}


/// Return the appropriate format string for the given settings.
fn make_format_string(settings: &Settings) -> &str {
    match settings.format {
        Format::Iso8601(ref fmt) => {
            match fmt {
                &Iso8601Format::Date => "%F",
                &Iso8601Format::Hours => "%FT%H%:z",
                &Iso8601Format::Minutes => "%FT%H:%M%:z",
                &Iso8601Format::Seconds => "%FT%T%:z",
                &Iso8601Format::Ns => "%FT%T,%f%:z",
            }
        }
        Format::Rfc2822 => "%a, %d %h %Y %T %z",
        Format::Rfc3339(ref fmt) => {
            match fmt {
                &Rfc3339Format::Date => "%F",
                &Rfc3339Format::Seconds => "%F %T%:z",
                &Rfc3339Format::Ns => "%F %T.%f%:z",
            }
        }
        Format::Custom(ref fmt) => fmt,
        Format::Default => "%c",
    }
}
