// spell-checker:ignore (format) DATEFILE

use clap::{crate_version, App, Arg};

// Options
pub const DATE: &str = "date";
pub const HOURS: &str = "hours";
pub const MINUTES: &str = "minutes";
pub const SECONDS: &str = "seconds";
pub const HOUR: &str = "hour";
pub const MINUTE: &str = "minute";
pub const SECOND: &str = "second";
pub const NS: &str = "ns";

pub const NAME: &str = "date";
pub const ABOUT: &str = "print or set the system date and time";

pub const OPT_DATE: &str = "date";
pub const OPT_FORMAT: &str = "format";
pub const OPT_FILE: &str = "file";
pub const OPT_DEBUG: &str = "debug";
pub const OPT_ISO_8601: &str = "iso-8601";
pub const OPT_RFC_EMAIL: &str = "rfc-email";
pub const OPT_RFC_3339: &str = "rfc-3339";
pub const OPT_SET: &str = "set";
pub const OPT_REFERENCE: &str = "reference";
pub const OPT_UNIVERSAL: &str = "universal";
pub const OPT_UNIVERSAL_2: &str = "utc";

// Help strings

const ISO_8601_HELP_STRING: &str = "output date/time in ISO 8601 format.
 FMT='date' for date only (the default),
 'hours', 'minutes', 'seconds', or 'ns'
 for date and time to the indicated precision.
 Example: 2006-08-14T02:34:56-06:00";

const RFC_5322_HELP_STRING: &str = "output date and time in RFC 5322 format.
 Example: Mon, 14 Aug 2006 02:34:56 -0600";

const RFC_3339_HELP_STRING: &str = "output date/time in RFC 3339 format.
 FMT='date', 'seconds', or 'ns'
 for date and time to the indicated precision.
 Example: 2006-08-14 02:34:56-06:00";

#[cfg(not(target_os = "macos"))]
const OPT_SET_HELP_STRING: &str = "set time described by STRING";
#[cfg(target_os = "macos")]
const OPT_SET_HELP_STRING: &str = "set time described by STRING (not available on mac yet)";

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .about(ABOUT)
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
                .help(OPT_SET_HELP_STRING),
        )
        .arg(
            Arg::with_name(OPT_UNIVERSAL)
                .short("u")
                .long(OPT_UNIVERSAL)
                .alias(OPT_UNIVERSAL_2)
                .help("print or set Coordinated Universal Time (UTC)"),
        )
        .arg(Arg::with_name(OPT_FORMAT).multiple(false))
}
