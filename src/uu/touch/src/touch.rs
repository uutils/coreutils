// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) filetime datetime lpszfilepath mktime DATETIME datelike timelike
// spell-checker:ignore (FORMATS) MMDDhhmm YYYYMMDDHHMM YYMMDDHHMM YYYYMMDDHHMMS

pub mod error;

use chrono::{
    DateTime, Datelike, Duration, Local, LocalResult, NaiveDate, NaiveDateTime, NaiveTime,
    TimeZone, Timelike,
};
use clap::builder::{PossibleValue, ValueParser};
use clap::{Arg, ArgAction, ArgGroup, ArgMatches, Command};
use filetime::{FileTime, set_file_times, set_symlink_file_times};
use jiff::{Timestamp, Zoned};
use std::borrow::Cow;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{Error, ErrorKind};
use std::path::{Path, PathBuf};
use uucore::display::Quotable;
use uucore::error::{FromIo, UResult, USimpleError};
use uucore::parser::shortcut_value_parser::ShortcutValueParser;
use uucore::translate;
use uucore::{format_usage, show};

use crate::error::TouchError;

/// Options contains all the possible behaviors and flags for touch.
///
/// All options are public so that the options can be programmatically
/// constructed by other crates, such as nushell. That means that this struct is
/// part of our public API. It should therefore not be changed without good reason.
///
/// The fields are documented with the arguments that determine their value.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Options {
    /// Do not create any files. Set by `-c`/`--no-create`.
    pub no_create: bool,

    /// Affect each symbolic link instead of any referenced file. Set by `-h`/`--no-dereference`.
    pub no_deref: bool,

    /// Where to get access and modification times from
    pub source: Source,

    /// If given, uses time from `source` but on given date
    pub date: Option<String>,

    /// Whether to change access time only, modification time only, or both
    pub change_times: ChangeTimes,

    /// When true, error when file doesn't exist and either `--no-dereference`
    /// was passed or the file couldn't be created
    pub strict: bool,
}

pub enum InputFile {
    /// A regular file
    Path(PathBuf),
    /// Touch stdout. `--no-dereference` will be ignored in this case.
    Stdout,
}

/// Whether to set access time only, modification time only, or both
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ChangeTimes {
    /// Change only access time
    AtimeOnly,
    /// Change only modification time
    MtimeOnly,
    /// Change both access and modification times
    Both,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Source {
    /// Use access/modification times of given file
    Reference(PathBuf),
    Timestamp(FileTime),
    /// Use current time
    Now,
}

pub mod options {
    // Both SOURCES and sources are needed as we need to be able to refer to the ArgGroup.
    pub static SOURCES: &str = "sources";
    pub mod sources {
        pub static DATE: &str = "date";
        pub static REFERENCE: &str = "reference";
        pub static TIMESTAMP: &str = "timestamp";
    }
    pub static HELP: &str = "help";
    pub static ACCESS: &str = "access";
    pub static MODIFICATION: &str = "modification";
    pub static NO_CREATE: &str = "no-create";
    pub static NO_DEREF: &str = "no-dereference";
    pub static TIME: &str = "time";
    pub static FORCE: &str = "force";
}

static ARG_FILES: &str = "files";

mod format {
    pub(crate) const POSIX_LOCALE: &str = "%a %b %e %H:%M:%S %Y";
    pub(crate) const ISO_8601: &str = "%Y-%m-%d";
    // "%Y%m%d%H%M.%S" 15 chars
    pub(crate) const YYYYMMDDHHMM_DOT_SS: &str = "%Y%m%d%H%M.%S";
    // "%Y-%m-%d %H:%M:%S.%SS" 12 chars
    pub(crate) const YYYYMMDDHHMMSS: &str = "%Y-%m-%d %H:%M:%S.%f";
    // "%Y-%m-%d %H:%M:%S" 12 chars
    pub(crate) const YYYYMMDDHHMMS: &str = "%Y-%m-%d %H:%M:%S";
    // "%Y-%m-%d %H:%M" 12 chars
    // Used for example in tests/touch/no-rights.sh
    pub(crate) const YYYY_MM_DD_HH_MM: &str = "%Y-%m-%d %H:%M";
    // "%Y%m%d%H%M" 12 chars
    pub(crate) const YYYYMMDDHHMM: &str = "%Y%m%d%H%M";
    // "%Y-%m-%d %H:%M +offset"
    // Used for example in tests/touch/relative.sh
    pub(crate) const YYYYMMDDHHMM_OFFSET: &str = "%Y-%m-%d %H:%M %z";
}

/// Convert a [`DateTime`] with a TZ offset into a [`FileTime`]
///
/// The [`DateTime`] is converted into a unix timestamp from which the [`FileTime`] is
/// constructed.
fn datetime_to_filetime<T: TimeZone>(dt: &DateTime<T>) -> FileTime {
    FileTime::from_unix_time(dt.timestamp(), dt.timestamp_subsec_nanos())
}

fn filetime_to_datetime(ft: &FileTime) -> Option<DateTime<Local>> {
    Some(DateTime::from_timestamp(ft.unix_seconds(), ft.nanoseconds())?.into())
}

/// Whether all characters in the string are digits.
fn all_digits(s: &str) -> bool {
    s.as_bytes().iter().all(u8::is_ascii_digit)
}

/// Convert a two-digit year string to the corresponding number.
///
/// `s` must be of length two or more. The last two bytes of `s` are
/// assumed to be the two digits of the year.
fn get_year(s: &str) -> u8 {
    let bytes = s.as_bytes();
    let n = bytes.len();
    let y1 = bytes[n - 2] - b'0';
    let y2 = bytes[n - 1] - b'0';
    10 * y1 + y2
}

/// Whether the first filename should be interpreted as a timestamp.
fn is_first_filename_timestamp(
    reference: Option<&OsString>,
    date: Option<&str>,
    timestamp: Option<&str>,
    files: &[&OsString],
) -> bool {
    timestamp.is_none()
        && reference.is_none()
        && date.is_none()
        && files.len() >= 2
        // env check is last as the slowest op
        && matches!(std::env::var("_POSIX2_VERSION").as_deref(), Ok("199209"))
        && files[0].to_str().is_some_and(is_timestamp)
}

// Check if string is a valid POSIX timestamp (8 digits or 10 digits with valid year range)
fn is_timestamp(s: &str) -> bool {
    all_digits(s) && (s.len() == 8 || (s.len() == 10 && (69..=99).contains(&get_year(s))))
}

/// Cycle the last two characters to the beginning of the string.
///
/// `s` must have length at least two.
fn shr2(s: &str) -> String {
    let n = s.len();
    let (a, b) = s.split_at(n - 2);
    let mut result = String::with_capacity(n);
    result.push_str(b);
    result.push_str(a);
    result
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;

    let mut filenames: Vec<&OsString> = matches
        .get_many::<OsString>(ARG_FILES)
        .ok_or_else(|| {
            USimpleError::new(
                1,
                translate!("touch-error-missing-file-operand", "help_command" => uucore::execution_phrase().to_string(),),
            )
        })?
        .collect();

    let no_deref = matches.get_flag(options::NO_DEREF);

    let reference = matches.get_one::<OsString>(options::sources::REFERENCE);
    let date = matches
        .get_one::<String>(options::sources::DATE)
        .map(|date| date.to_owned());

    let mut timestamp = matches
        .get_one::<String>(options::sources::TIMESTAMP)
        .map(|t| t.to_owned());

    if is_first_filename_timestamp(reference, date.as_deref(), timestamp.as_deref(), &filenames) {
        let first_file = filenames[0].to_str().unwrap();
        timestamp = if first_file.len() == 10 {
            Some(shr2(first_file))
        } else {
            Some(first_file.to_string())
        };
        filenames = filenames[1..].to_vec();
    }

    let source = if let Some(reference) = reference {
        Source::Reference(PathBuf::from(reference))
    } else if let Some(ts) = timestamp {
        Source::Timestamp(parse_timestamp(&ts)?)
    } else {
        Source::Now
    };

    let files: Vec<InputFile> = filenames
        .into_iter()
        .map(|filename| {
            if filename == "-" {
                InputFile::Stdout
            } else {
                InputFile::Path(PathBuf::from(filename))
            }
        })
        .collect();

    let opts = Options {
        no_create: matches.get_flag(options::NO_CREATE),
        no_deref,
        source,
        date,
        change_times: determine_atime_mtime_change(&matches),
        strict: false,
    };

    touch(&files, &opts)?;

    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .about(translate!("touch-about"))
        .override_usage(format_usage(&translate!("touch-usage")))
        .infer_long_args(true)
        .disable_help_flag(true)
        .arg(
            Arg::new(options::HELP)
                .long(options::HELP)
                .help(translate!("touch-help-help"))
                .action(ArgAction::Help),
        )
        .arg(
            Arg::new(options::ACCESS)
                .short('a')
                .help(translate!("touch-help-access"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::sources::TIMESTAMP)
                .short('t')
                .help(translate!("touch-help-timestamp"))
                .value_name("STAMP"),
        )
        .arg(
            Arg::new(options::sources::DATE)
                .short('d')
                .long(options::sources::DATE)
                .allow_hyphen_values(true)
                .help(translate!("touch-help-date"))
                .value_name("STRING")
                .conflicts_with(options::sources::TIMESTAMP),
        )
        .arg(
            Arg::new(options::FORCE)
                .short('f')
                .help("(ignored)")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::MODIFICATION)
                .short('m')
                .help(translate!("touch-help-modification"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::NO_CREATE)
                .short('c')
                .long(options::NO_CREATE)
                .help(translate!("touch-help-no-create"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::NO_DEREF)
                .short('h')
                .long(options::NO_DEREF)
                .help(translate!("touch-help-no-deref"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::sources::REFERENCE)
                .short('r')
                .long(options::sources::REFERENCE)
                .help(translate!("touch-help-reference"))
                .value_name("FILE")
                .value_parser(ValueParser::os_string())
                .value_hint(clap::ValueHint::AnyPath)
                .conflicts_with(options::sources::TIMESTAMP),
        )
        .arg(
            Arg::new(options::TIME)
                .long(options::TIME)
                .help(translate!("touch-help-time"))
                .value_name("WORD")
                .value_parser(ShortcutValueParser::new([
                    PossibleValue::new("atime").alias("access").alias("use"),
                    PossibleValue::new("mtime").alias("modify"),
                ])),
        )
        .arg(
            Arg::new(ARG_FILES)
                .action(ArgAction::Append)
                .num_args(1..)
                .value_parser(clap::value_parser!(OsString))
                .value_hint(clap::ValueHint::AnyPath),
        )
        .group(
            ArgGroup::new(options::SOURCES)
                .args([
                    options::sources::TIMESTAMP,
                    options::sources::DATE,
                    options::sources::REFERENCE,
                ])
                .multiple(true),
        )
}

/// Execute the touch command.
///
/// # Errors
///
/// Possible causes:
/// - The user doesn't have permission to access the file
/// - One of the directory components of the file path doesn't exist.
///
/// It will return an `Err` on the first error. However, for any of the files,
/// if all of the following are true, it will print the error and continue touching
/// the rest of the files.
/// - `opts.strict` is `false`
/// - The file doesn't already exist
/// - `-c`/`--no-create` was passed (`opts.no_create`)
/// - Either `-h`/`--no-dereference` was passed (`opts.no_deref`) or the file couldn't be created
pub fn touch(files: &[InputFile], opts: &Options) -> Result<(), TouchError> {
    let (atime, mtime) = match &opts.source {
        Source::Reference(reference) => {
            let (atime, mtime) = stat(reference, !opts.no_deref)
                .map_err(|e| TouchError::ReferenceFileInaccessible(reference.to_owned(), e))?;

            (atime, mtime)
        }
        Source::Now => {
            let now = datetime_to_filetime(&Local::now());
            (now, now)
        }
        &Source::Timestamp(ts) => (ts, ts),
    };

    let (atime, mtime) = if let Some(date) = &opts.date {
        (
            parse_date(
                filetime_to_datetime(&atime).ok_or_else(|| TouchError::InvalidFiletime(atime))?,
                date,
            )?,
            parse_date(
                filetime_to_datetime(&mtime).ok_or_else(|| TouchError::InvalidFiletime(mtime))?,
                date,
            )?,
        )
    } else {
        (atime, mtime)
    };

    for (ind, file) in files.iter().enumerate() {
        let (path, is_stdout) = match file {
            InputFile::Stdout => (Cow::Owned(pathbuf_from_stdout()?), true),
            InputFile::Path(path) => (Cow::Borrowed(path), false),
        };
        touch_file(&path, is_stdout, opts, atime, mtime).map_err(|e| {
            TouchError::TouchFileError {
                path: path.into_owned(),
                index: ind,
                error: e,
            }
        })?;
    }

    Ok(())
}

/// Create or update the timestamp for a single file.
///
/// # Arguments
///
/// - `path` - The path to the file to create/update timestamp for
/// - `is_stdout` - Stdout is handled specially, see [`update_times`] for more info
/// - `atime` - Access time to set for the file
/// - `mtime` - Modification time to set for the file
fn touch_file(
    path: &Path,
    is_stdout: bool,
    opts: &Options,
    atime: FileTime,
    mtime: FileTime,
) -> UResult<()> {
    let filename = if is_stdout {
        String::from("-")
    } else {
        path.display().to_string()
    };

    let metadata_result = if opts.no_deref {
        path.symlink_metadata()
    } else {
        path.metadata()
    };

    if let Err(e) = metadata_result {
        if e.kind() != ErrorKind::NotFound {
            return Err(e.map_err_context(
                || translate!("touch-error-setting-times-of", "filename" => filename.quote()),
            ));
        }

        if opts.no_create {
            return Ok(());
        }

        if opts.no_deref {
            let e = USimpleError::new(
                1,
                translate!("touch-error-setting-times-no-such-file", "filename" => filename.quote()),
            );
            if opts.strict {
                return Err(e);
            }
            show!(e);
            return Ok(());
        }

        if let Err(e) = File::create(path) {
            // we need to check if the path is the path to a directory (ends with a separator)
            // we can't use File::create to create a directory
            // we cannot use path.is_dir() because it calls fs::metadata which we already called
            // when stable, we can change to use e.kind() == std::io::ErrorKind::IsADirectory
            let is_directory = if let Some(last_char) = path.to_string_lossy().chars().last() {
                last_char == std::path::MAIN_SEPARATOR
            } else {
                false
            };
            if is_directory {
                let custom_err = Error::other(translate!("touch-error-no-such-file-or-directory"));
                return Err(custom_err.map_err_context(
                    || translate!("touch-error-cannot-touch", "filename" => filename.quote()),
                ));
            }
            let e = e.map_err_context(
                || translate!("touch-error-cannot-touch", "filename" => path.quote()),
            );
            if opts.strict {
                return Err(e);
            }
            show!(e);
            return Ok(());
        }

        // Minor optimization: if no reference time, timestamp, or date was specified, we're done.
        if opts.source == Source::Now && opts.date.is_none() {
            return Ok(());
        }
    }

    update_times(path, is_stdout, opts, atime, mtime)
}

/// Returns which of the times (access, modification) are to be changed.
///
/// Note that "-a" and "-m" may be passed together; this is not an xor.
/// - If `-a` is passed but not `-m`, only access time is changed
/// - If `-m` is passed but not `-a`, only modification time is changed
/// - If neither or both are passed, both times are changed
fn determine_atime_mtime_change(matches: &ArgMatches) -> ChangeTimes {
    // If `--time` is given, Some(true) if equivalent to `-a`, Some(false) if equivalent to `-m`
    // If `--time` not given, None
    let time_access_only = if matches.contains_id(options::TIME) {
        matches
            .get_one::<String>(options::TIME)
            .map(|time| time.contains("access") || time.contains("atime") || time.contains("use"))
    } else {
        None
    };

    let atime_only = matches.get_flag(options::ACCESS) || time_access_only.unwrap_or_default();
    let mtime_only = matches.get_flag(options::MODIFICATION) || !time_access_only.unwrap_or(true);

    if atime_only && !mtime_only {
        ChangeTimes::AtimeOnly
    } else if mtime_only && !atime_only {
        ChangeTimes::MtimeOnly
    } else {
        ChangeTimes::Both
    }
}

/// Updating file access and modification times based on user-specified options
///
/// If the file is not stdout (`!is_stdout`) and `-h`/`--no-dereference` was
/// passed, then, if the given file is a symlink, its own times will be updated,
/// rather than the file it points to.
fn update_times(
    path: &Path,
    is_stdout: bool,
    opts: &Options,
    atime: FileTime,
    mtime: FileTime,
) -> UResult<()> {
    // If changing "only" atime or mtime, grab the existing value of the other.
    let (atime, mtime) = match opts.change_times {
        ChangeTimes::AtimeOnly => (
            atime,
            stat(path, !opts.no_deref)
                .map_err_context(
                    || translate!("touch-error-failed-to-get-attributes", "path" => path.quote()),
                )?
                .1,
        ),
        ChangeTimes::MtimeOnly => (
            stat(path, !opts.no_deref)
                .map_err_context(
                    || translate!("touch-error-failed-to-get-attributes", "path" => path.quote()),
                )?
                .0,
            mtime,
        ),
        ChangeTimes::Both => (atime, mtime),
    };

    // sets the file access and modification times for a file or a symbolic link.
    // The filename, access time (atime), and modification time (mtime) are provided as inputs.

    if opts.no_deref && !is_stdout {
        set_symlink_file_times(path, atime, mtime)
    } else {
        set_file_times(path, atime, mtime)
    }
    .map_err_context(|| translate!("touch-error-setting-times-of-path", "path" => path.quote()))
}

/// Get metadata of the provided path
/// If `follow` is `true`, the function will try to follow symlinks
/// If `follow` is `false` or the symlink is broken, the function will return metadata of the symlink itself
fn stat(path: &Path, follow: bool) -> std::io::Result<(FileTime, FileTime)> {
    let metadata = if follow {
        fs::metadata(path).or_else(|_| fs::symlink_metadata(path))
    } else {
        fs::symlink_metadata(path)
    }?;

    Ok((
        FileTime::from_last_access_time(&metadata),
        FileTime::from_last_modification_time(&metadata),
    ))
}

fn parse_date(ref_time: DateTime<Local>, s: &str) -> Result<FileTime, TouchError> {
    // This isn't actually compatible with GNU touch, but there doesn't seem to
    // be any simple specification for what format this parameter allows and I'm
    // not about to implement GNU parse_datetime.
    // http://git.savannah.gnu.org/gitweb/?p=gnulib.git;a=blob_plain;f=lib/parse-datetime.y

    // TODO: match on char count?

    // "The preferred date and time representation for the current locale."
    // "(In the POSIX locale this is equivalent to %a %b %e %H:%M:%S %Y.)"
    // time 0.1.43 parsed this as 'a b e T Y'
    // which is equivalent to the POSIX locale: %a %b %e %H:%M:%S %Y
    // Tue Dec  3 ...
    // ("%c", POSIX_LOCALE_FORMAT),
    //
    if let Ok(parsed) = NaiveDateTime::parse_from_str(s, format::POSIX_LOCALE) {
        return Ok(datetime_to_filetime(&parsed.and_utc()));
    }

    // Also support other formats found in the GNU tests like
    // in tests/misc/stat-nanoseconds.sh
    // or tests/touch/no-rights.sh
    for fmt in [
        format::YYYYMMDDHHMMS,
        format::YYYYMMDDHHMMSS,
        format::YYYY_MM_DD_HH_MM,
        format::YYYYMMDDHHMM_OFFSET,
    ] {
        if let Ok(parsed) = NaiveDateTime::parse_from_str(s, fmt) {
            return Ok(datetime_to_filetime(&parsed.and_utc()));
        }
    }

    // "Equivalent to %Y-%m-%d (the ISO 8601 date format). (C99)"
    // ("%F", ISO_8601_FORMAT),
    if let Ok(parsed_date) = NaiveDate::parse_from_str(s, format::ISO_8601) {
        let parsed = Local
            .from_local_datetime(&parsed_date.and_time(NaiveTime::MIN))
            .unwrap();
        return Ok(datetime_to_filetime(&parsed));
    }

    // "@%s" is "The number of seconds since the Epoch, 1970-01-01 00:00:00 +0000 (UTC). (TZ) (Calculated from mktime(tm).)"
    if s.bytes().next() == Some(b'@') {
        if let Ok(ts) = &s[1..].parse::<i64>() {
            return Ok(FileTime::from_unix_time(*ts, 0));
        }
    }

    // **parse_datetime 0.13 API change:**
    // The parse_datetime crate was updated from 0.11 to 0.13 in commit 2a69918ca to fix
    // issue #8754 (large second values like "12345.123456789 seconds ago" failing).
    // This introduced a breaking API change in parse_datetime_at_date:
    //
    // Previously (0.11): parse_datetime_at_date(chrono::DateTime) → chrono::DateTime
    // Now (0.13):        parse_datetime_at_date(jiff::Zoned) → jiff::Zoned
    //
    // Commit 4340913c4 initially adapted to this by switching from parse_datetime_at_date
    // to parse_datetime, which broke deterministic relative date parsing (the ref_time
    // parameter was no longer used, causing tests/touch/relative to fail in CI).
    //
    // This implementation restores parse_datetime_at_date usage with proper conversions:
    // chrono::DateTime → jiff::Zoned → parse_datetime_at_date → jiff::Zoned → chrono::DateTime
    //
    // The use of parse_datetime_at_date (not parse_datetime) is critical for deterministic
    // behavior with relative dates like "yesterday" or "2 days ago", which must be
    // calculated relative to ref_time, not the current system time.

    // Convert chrono DateTime to jiff Zoned for parse_datetime_at_date
    let ref_zoned = {
        let ts = Timestamp::new(
            ref_time.timestamp(),
            ref_time.timestamp_subsec_nanos() as i32,
        )
        .map_err(|_| TouchError::InvalidDateFormat(s.to_owned()))?;
        Zoned::new(ts, jiff::tz::TimeZone::system())
    };

    if let Ok(zoned) = parse_datetime::parse_datetime_at_date(ref_zoned, s) {
        let timestamp = zoned.timestamp();
        let dt =
            DateTime::from_timestamp(timestamp.as_second(), timestamp.subsec_nanosecond() as u32)
                .map(|dt| dt.with_timezone(&Local))
                .ok_or_else(|| TouchError::InvalidDateFormat(s.to_owned()))?;
        return Ok(datetime_to_filetime(&dt));
    }

    Err(TouchError::InvalidDateFormat(s.to_owned()))
}

/// Prepends 19 or 20 to the year if it is a 2 digit year
///
/// GNU `touch` behavior:
///
/// - 68 and before is interpreted as 20xx
/// - 69 and after is interpreted as 19xx
fn prepend_century(s: &str) -> UResult<String> {
    let first_two_digits = s[..2].parse::<u32>().map_err(|_| {
        USimpleError::new(
            1,
            translate!("touch-error-invalid-date-ts-format", "date" => s.quote()),
        )
    })?;
    Ok(format!(
        "{}{s}",
        if first_two_digits > 68 { 19 } else { 20 }
    ))
}

/// Parses a timestamp string into a [`FileTime`].
///
/// This function attempts to parse a string into a [`FileTime`]
/// As expected by gnu touch -t : `[[cc]yy]mmddhhmm[.ss]`
///
/// Note that  If the year is specified with only two digits,
/// then cc is 20 for years in the range 0 … 68, and 19 for years in 69 … 99.
/// in order to be compatible with GNU `touch`.
fn parse_timestamp(s: &str) -> UResult<FileTime> {
    use format::*;

    let current_year = || Local::now().year();

    let (format, ts) = match s.chars().count() {
        15 => (YYYYMMDDHHMM_DOT_SS, s.to_owned()),
        12 => (YYYYMMDDHHMM, s.to_owned()),
        // If we don't add "19" or "20", we have insufficient information to parse
        13 => (YYYYMMDDHHMM_DOT_SS, prepend_century(s)?),
        10 => (YYYYMMDDHHMM, prepend_century(s)?),
        11 => (YYYYMMDDHHMM_DOT_SS, format!("{}{s}", current_year())),
        8 => (YYYYMMDDHHMM, format!("{}{s}", current_year())),
        _ => {
            return Err(USimpleError::new(
                1,
                translate!("touch-error-invalid-date-format", "date" => s.quote()),
            ));
        }
    };

    let local = NaiveDateTime::parse_from_str(&ts, format).map_err(|_| {
        USimpleError::new(
            1,
            translate!("touch-error-invalid-date-ts-format", "date" => ts.quote()),
        )
    })?;
    let LocalResult::Single(mut local) = Local.from_local_datetime(&local) else {
        return Err(USimpleError::new(
            1,
            translate!("touch-error-invalid-date-ts-format", "date" => ts.quote()),
        ));
    };

    // Chrono caps seconds at 59, but 60 is valid. It might be a leap second
    // or wrap to the next minute. But that doesn't really matter, because we
    // only care about the timestamp anyway.
    // Tested in gnu/tests/touch/60-seconds
    if local.second() == 59 && ts.ends_with(".60") {
        local += Duration::try_seconds(1).unwrap();
    }

    // Due to daylight saving time switch, local time can jump from 1:59 AM to
    // 3:00 AM, in which case any time between 2:00 AM and 2:59 AM is not
    // valid. If we are within this jump, chrono takes the offset from before
    // the jump. If we then jump forward an hour, we get the new corrected
    // offset. Jumping back will then now correctly take the jump into account.
    let local2 = local + Duration::try_hours(1).unwrap() - Duration::try_hours(1).unwrap();
    if local.hour() != local2.hour() {
        return Err(USimpleError::new(
            1,
            translate!("touch-error-invalid-date-format", "date" => s.quote()),
        ));
    }

    Ok(datetime_to_filetime(&local))
}

// TODO: this may be a good candidate to put in fsext.rs
/// Returns a [`PathBuf`] to stdout.
///
/// On Windows, uses `GetFinalPathNameByHandleW` to attempt to get the path
/// from the stdout handle.
fn pathbuf_from_stdout() -> Result<PathBuf, TouchError> {
    #[cfg(all(unix, not(target_os = "android")))]
    {
        Ok(PathBuf::from("/dev/stdout"))
    }
    #[cfg(target_os = "android")]
    {
        Ok(PathBuf::from("/proc/self/fd/1"))
    }
    #[cfg(windows)]
    {
        use std::os::windows::prelude::AsRawHandle;
        use windows_sys::Win32::Foundation::{
            ERROR_INVALID_PARAMETER, ERROR_NOT_ENOUGH_MEMORY, ERROR_PATH_NOT_FOUND, GetLastError,
            HANDLE, MAX_PATH,
        };
        use windows_sys::Win32::Storage::FileSystem::{
            FILE_NAME_OPENED, GetFinalPathNameByHandleW,
        };

        let handle = std::io::stdout().lock().as_raw_handle() as HANDLE;
        let mut file_path_buffer: [u16; MAX_PATH as usize] = [0; MAX_PATH as usize];

        // https://docs.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getfinalpathnamebyhandlea#examples
        // SAFETY: We transmute the handle to be able to cast *mut c_void into a
        // HANDLE (i32) so rustc will let us call GetFinalPathNameByHandleW. The
        // reference example code for GetFinalPathNameByHandleW implies that
        // it is safe for us to leave lpszfilepath uninitialized, so long as
        // the buffer size is correct. We know the buffer size (MAX_PATH) at
        // compile time. MAX_PATH is a small number (260) so we can cast it
        // to a u32.
        let ret = unsafe {
            GetFinalPathNameByHandleW(
                handle,
                file_path_buffer.as_mut_ptr(),
                file_path_buffer.len() as u32,
                FILE_NAME_OPENED,
            )
        };

        let buffer_size = match ret {
            ERROR_PATH_NOT_FOUND | ERROR_NOT_ENOUGH_MEMORY | ERROR_INVALID_PARAMETER => {
                return Err(TouchError::WindowsStdoutPathError(
                    translate!("touch-error-windows-stdout-path-failed", "code" => ret),
                ));
            }
            0 => {
                return Err(TouchError::WindowsStdoutPathError(translate!(
                "touch-error-windows-stdout-path-failed",
                    "code".to_string() =>
                    format!(
                        "{}",
                        // SAFETY: GetLastError is thread-safe and has no documented memory unsafety.
                        unsafe { GetLastError() }
                    ),
                )));
            }
            e => e as usize,
        };

        // Don't include the null terminator
        Ok(String::from_utf16(&file_path_buffer[0..buffer_size])
            .map_err(|e| TouchError::WindowsStdoutPathError(e.to_string()))?
            .into())
    }
}

#[cfg(test)]
mod tests {
    use filetime::FileTime;

    use crate::{
        ChangeTimes, Options, Source, determine_atime_mtime_change, error::TouchError, touch,
        uu_app,
    };

    #[cfg(windows)]
    use std::env;
    #[cfg(windows)]
    use uucore::locale;

    #[cfg(windows)]
    #[test]
    fn test_get_pathbuf_from_stdout_fails_if_stdout_is_not_a_file() {
        unsafe {
            env::set_var("LANG", "C");
        }
        let _ = locale::setup_localization("touch");
        // We can trigger an error by not setting stdout to anything (will
        // fail with code 1)
        assert!(
            super::pathbuf_from_stdout()
                .expect_err("pathbuf_from_stdout should have failed")
                .to_string()
                .contains("GetFinalPathNameByHandleW failed with code 1")
        );
    }

    #[test]
    fn test_determine_atime_mtime_change() {
        assert_eq!(
            ChangeTimes::Both,
            determine_atime_mtime_change(&uu_app().try_get_matches_from(vec!["touch"]).unwrap())
        );
        assert_eq!(
            ChangeTimes::Both,
            determine_atime_mtime_change(
                &uu_app()
                    .try_get_matches_from(vec!["touch", "-a", "-m", "--time", "modify"])
                    .unwrap()
            )
        );
        assert_eq!(
            ChangeTimes::AtimeOnly,
            determine_atime_mtime_change(
                &uu_app()
                    .try_get_matches_from(vec!["touch", "--time", "access"])
                    .unwrap()
            )
        );
        assert_eq!(
            ChangeTimes::MtimeOnly,
            determine_atime_mtime_change(
                &uu_app().try_get_matches_from(vec!["touch", "-m"]).unwrap()
            )
        );
    }

    #[test]
    fn test_invalid_filetime() {
        let invalid_filetime = FileTime::from_unix_time(0, 1_111_111_111);
        match touch(
            &[],
            &Options {
                no_create: false,
                no_deref: false,
                source: Source::Timestamp(invalid_filetime),
                date: Some("yesterday".to_owned()),
                change_times: ChangeTimes::Both,
                strict: false,
            },
        ) {
            Err(TouchError::InvalidFiletime(filetime)) => assert_eq!(filetime, invalid_filetime),
            Err(e) => panic!("Expected TouchError::InvalidFiletime, got {e}"),
            Ok(_) => panic!("Expected to error with TouchError::InvalidFiletime but succeeded"),
        }
    }
}
