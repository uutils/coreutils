// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{crate_version, Arg, ArgAction, Command};

use uucore::shortcut_value_parser::ShortcutValueParser;
use uucore::{format_usage, help_about, help_usage};

const ABOUT: &str = help_about!("date.md");
const USAGE: &str = help_usage!("date.md");

pub mod options {
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

    // Options
    pub const DATE: &str = "date";
    pub const HOURS: &str = "hours";
    pub const MINUTES: &str = "minutes";
    pub const SECONDS: &str = "seconds";
    pub const NS: &str = "ns";

    // Help strings

    pub static ISO_8601_HELP_STRING: &str = "output date/time in ISO 8601 format.
FMT='date' for date only (the default),
'hours', 'minutes', 'seconds', or 'ns'
for date and time to the indicated precision.
Example: 2006-08-14T02:34:56-06:00";

    pub static RFC_5322_HELP_STRING: &str = "output date and time in RFC 5322 format.
Example: Mon, 14 Aug 2006 02:34:56 -0600";

    pub static RFC_3339_HELP_STRING: &str = "output date/time in RFC 3339 format.
FMT='date', 'seconds', or 'ns'
for date and time to the indicated precision.
Example: 2006-08-14 02:34:56-06:00";

    #[cfg(not(any(target_os = "macos", target_os = "redox")))]
    pub static OPT_SET_HELP_STRING: &str = "set time described by STRING";
    #[cfg(target_os = "macos")]
    pub static OPT_SET_HELP_STRING: &str =
        "set time described by STRING (not available on mac yet)";
    #[cfg(target_os = "redox")]
    pub static OPT_SET_HELP_STRING: &str =
        "set time described by STRING (not available on redox yet)";
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::OPT_DATE)
                .short('d')
                .long(options::OPT_DATE)
                .value_name("STRING")
                .help("display time described by STRING, not 'now'"),
        )
        .arg(
            Arg::new(options::OPT_FILE)
                .short('f')
                .long(options::OPT_FILE)
                .value_name("DATEFILE")
                .value_hint(clap::ValueHint::FilePath)
                .help("like --date; once for each line of DATEFILE"),
        )
        .arg(
            Arg::new(options::OPT_ISO_8601)
                .short('I')
                .long(options::OPT_ISO_8601)
                .value_name("FMT")
                .value_parser(ShortcutValueParser::new([
                    options::DATE,
                    options::HOURS,
                    options::MINUTES,
                    options::SECONDS,
                    options::NS,
                ]))
                .num_args(0..=1)
                .default_missing_value(options::OPT_DATE)
                .help(options::ISO_8601_HELP_STRING),
        )
        .arg(
            Arg::new(options::OPT_RFC_EMAIL)
                .short('R')
                .long(options::OPT_RFC_EMAIL)
                .help(options::RFC_5322_HELP_STRING)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_RFC_3339)
                .long(options::OPT_RFC_3339)
                .value_name("FMT")
                .value_parser(ShortcutValueParser::new([
                    options::DATE,
                    options::SECONDS,
                    options::NS,
                ]))
                .help(options::RFC_3339_HELP_STRING),
        )
        .arg(
            Arg::new(options::OPT_DEBUG)
                .long(options::OPT_DEBUG)
                .help("annotate the parsed date, and warn about questionable usage to stderr")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_REFERENCE)
                .short('r')
                .long(options::OPT_REFERENCE)
                .value_name("FILE")
                .value_hint(clap::ValueHint::AnyPath)
                .help("display the last modification time of FILE"),
        )
        .arg(
            Arg::new(options::OPT_SET)
                .short('s')
                .long(options::OPT_SET)
                .value_name("STRING")
                .help(options::OPT_SET_HELP_STRING),
        )
        .arg(
            Arg::new(options::OPT_UNIVERSAL)
                .short('u')
                .long(options::OPT_UNIVERSAL)
                .alias(options::OPT_UNIVERSAL_2)
                .help("print or set Coordinated Universal Time (UTC)")
                .action(ArgAction::SetTrue),
        )
        .arg(Arg::new(options::OPT_FORMAT))
}
