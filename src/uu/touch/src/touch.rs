// This file is part of the uutils coreutils package.
//
// (c) Nick Platt <platt.nicholas@gmail.com>
// (c) Jian Zeng <anonymousknight96 AT gmail.com>
//
// For the full copyright and license information, please view the LICENSE file
// that was distributed with this source code.

// spell-checker:ignore (ToDO) filetime datetime MMDDhhmm lpszfilepath mktime YYYYMMDDHHMM YYMMDDHHMM DATETIME YYYYMMDDHHMMS subsecond humantime

use clap::builder::ValueParser;
use clap::{crate_version, Arg, ArgAction, ArgGroup, Command};
use filetime::{set_symlink_file_times, FileTime};
use std::ffi::OsString;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use time::macros::{format_description, offset, time};
use time::Duration;
use uucore::display::Quotable;
use uucore::error::{FromIo, UError, UResult, USimpleError};
use uucore::{format_usage, help_about, help_usage, show};

const ABOUT: &str = help_about!("touch.md");
const USAGE: &str = help_usage!("touch.md");

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
}

static ARG_FILES: &str = "files";

// Convert a date/time to a date with a TZ offset
fn to_local(tm: time::PrimitiveDateTime) -> time::OffsetDateTime {
    let offset = match time::OffsetDateTime::now_local() {
        Ok(lo) => lo.offset(),
        Err(e) => {
            panic!("error: {e}");
        }
    };
    tm.assume_offset(offset)
}

// Convert a date/time with a TZ offset into a FileTime
fn local_dt_to_filetime(dt: time::OffsetDateTime) -> FileTime {
    FileTime::from_unix_time(dt.unix_timestamp(), dt.nanosecond())
}

// Convert a date/time, considering that the input is in UTC time
// Used for touch -d 1970-01-01 18:43:33.023456789 for example
fn dt_to_filename(tm: time::PrimitiveDateTime) -> FileTime {
    let dt = tm.assume_offset(offset!(UTC));
    local_dt_to_filetime(dt)
}

#[uucore::main]
#[allow(clippy::cognitive_complexity)]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().try_get_matches_from(args)?;

    let files = matches.get_many::<OsString>(ARG_FILES).ok_or_else(|| {
        USimpleError::new(
            1,
            format!(
                "missing file operand\nTry '{} --help' for more information.",
                uucore::execution_phrase()
            ),
        )
    })?;
    let (mut atime, mut mtime) = match (
        matches.get_one::<OsString>(options::sources::REFERENCE),
        matches.get_one::<String>(options::sources::DATE),
    ) {
        (Some(reference), Some(date)) => {
            let (atime, mtime) = stat(Path::new(reference), !matches.get_flag(options::NO_DEREF))?;
            if let Ok(offset) = parse_datetime::from_str(date) {
                let seconds = offset.num_seconds();
                let nanos = offset.num_nanoseconds().unwrap_or(0) % 1_000_000_000;

                let ref_atime_secs = atime.unix_seconds();
                let ref_atime_nanos = atime.nanoseconds();
                let atime = FileTime::from_unix_time(
                    ref_atime_secs + seconds,
                    ref_atime_nanos + nanos as u32,
                );

                let ref_mtime_secs = mtime.unix_seconds();
                let ref_mtime_nanos = mtime.nanoseconds();
                let mtime = FileTime::from_unix_time(
                    ref_mtime_secs + seconds,
                    ref_mtime_nanos + nanos as u32,
                );

                (atime, mtime)
            } else {
                let timestamp = parse_date(date)?;
                (timestamp, timestamp)
            }
        }
        (Some(reference), None) => {
            stat(Path::new(reference), !matches.get_flag(options::NO_DEREF))?
        }
        (None, Some(date)) => {
            let timestamp = parse_date(date)?;
            (timestamp, timestamp)
        }
        (None, None) => {
            let timestamp = if let Some(ts) = matches.get_one::<String>(options::sources::TIMESTAMP)
            {
                parse_timestamp(ts)?
            } else {
                local_dt_to_filetime(time::OffsetDateTime::now_local().unwrap())
            };
            (timestamp, timestamp)
        }
    };

    for filename in files {
        // FIXME: find a way to avoid having to clone the path
        let pathbuf = if filename == "-" {
            pathbuf_from_stdout()?
        } else {
            PathBuf::from(filename)
        };

        let path = pathbuf.as_path();

        let metadata_result = if matches.get_flag(options::NO_DEREF) {
            path.symlink_metadata()
        } else {
            path.metadata()
        };

        if let Err(e) = metadata_result {
            if e.kind() != std::io::ErrorKind::NotFound {
                return Err(e.map_err_context(|| format!("setting times of {}", filename.quote())));
            }

            if matches.get_flag(options::NO_CREATE) {
                continue;
            }

            if matches.get_flag(options::NO_DEREF) {
                show!(USimpleError::new(
                    1,
                    format!(
                        "setting times of {}: No such file or directory",
                        filename.quote()
                    )
                ));
                continue;
            }

            if let Err(e) = File::create(path) {
                show!(e.map_err_context(|| format!("cannot touch {}", path.quote())));
                continue;
            };

            // Minor optimization: if no reference time was specified, we're done.
            if !matches.contains_id(options::SOURCES) {
                continue;
            }
        }

        // If changing "only" atime or mtime, grab the existing value of the other.
        // Note that "-a" and "-m" may be passed together; this is not an xor.
        if matches.get_flag(options::ACCESS)
            || matches.get_flag(options::MODIFICATION)
            || matches.contains_id(options::TIME)
        {
            let st = stat(path, !matches.get_flag(options::NO_DEREF))?;
            let time = matches
                .get_one::<String>(options::TIME)
                .map(|s| s.as_str())
                .unwrap_or("");

            if !(matches.get_flag(options::ACCESS)
                || time.contains(&"access".to_owned())
                || time.contains(&"atime".to_owned())
                || time.contains(&"use".to_owned()))
            {
                atime = st.0;
            }

            if !(matches.get_flag(options::MODIFICATION)
                || time.contains(&"modify".to_owned())
                || time.contains(&"mtime".to_owned()))
            {
                mtime = st.1;
            }
        }

        // sets the file access and modification times for a file or a symbolic link.
        // The filename, access time (atime), and modification time (mtime) are provided as inputs.

        // If the filename is not "-", indicating a special case for touch -h -,
        // the code checks if the NO_DEREF flag is set, which means the user wants to
        // set the times for a symbolic link itself, rather than the file it points to.
        if filename == "-" {
            filetime::set_file_times(path, atime, mtime)
        } else if matches.get_flag(options::NO_DEREF) {
            set_symlink_file_times(path, atime, mtime)
        } else {
            filetime::set_file_times(path, atime, mtime)
        }
        .map_err_context(|| format!("setting times of {}", path.quote()))?;
    }
    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .disable_help_flag(true)
        .arg(
            Arg::new(options::HELP)
                .long(options::HELP)
                .help("Print help information.")
                .action(ArgAction::Help),
        )
        .arg(
            Arg::new(options::ACCESS)
                .short('a')
                .help("change only the access time")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::sources::TIMESTAMP)
                .short('t')
                .help("use [[CC]YY]MMDDhhmm[.ss] instead of the current time")
                .value_name("STAMP"),
        )
        .arg(
            Arg::new(options::sources::DATE)
                .short('d')
                .long(options::sources::DATE)
                .allow_hyphen_values(true)
                .help("parse argument and use it instead of current time")
                .value_name("STRING")
                .conflicts_with(options::sources::TIMESTAMP),
        )
        .arg(
            Arg::new(options::MODIFICATION)
                .short('m')
                .help("change only the modification time")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::NO_CREATE)
                .short('c')
                .long(options::NO_CREATE)
                .help("do not create any files")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::NO_DEREF)
                .short('h')
                .long(options::NO_DEREF)
                .help(
                    "affect each symbolic link instead of any referenced file \
                     (only for systems that can change the timestamps of a symlink)",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::sources::REFERENCE)
                .short('r')
                .long(options::sources::REFERENCE)
                .help("use this file's times instead of the current time")
                .value_name("FILE")
                .value_parser(ValueParser::os_string())
                .value_hint(clap::ValueHint::AnyPath)
                .conflicts_with(options::sources::TIMESTAMP),
        )
        .arg(
            Arg::new(options::TIME)
                .long(options::TIME)
                .help(
                    "change only the specified time: \"access\", \"atime\", or \
                     \"use\" are equivalent to -a; \"modify\" or \"mtime\" are \
                     equivalent to -m",
                )
                .value_name("WORD")
                .value_parser(["access", "atime", "use"]),
        )
        .arg(
            Arg::new(ARG_FILES)
                .action(ArgAction::Append)
                .num_args(1..)
                .value_parser(ValueParser::os_string())
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

// Get metadata of the provided path
// If `follow` is `true`, the function will try to follow symlinks
// If `follow` is `false` or the symlink is broken, the function will return metadata of the symlink itself
fn stat(path: &Path, follow: bool) -> UResult<(FileTime, FileTime)> {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound && !follow => fs::symlink_metadata(path)
            .map_err_context(|| format!("failed to get attributes of {}", path.quote()))?,
        Err(e) => return Err(e.into()),
    };

    Ok((
        FileTime::from_last_access_time(&metadata),
        FileTime::from_last_modification_time(&metadata),
    ))
}

const POSIX_LOCALE_FORMAT: &[time::format_description::FormatItem] = format_description!(
    "[weekday repr:short] [month repr:short] [day padding:space] \
    [hour]:[minute]:[second] [year]"
);

const ISO_8601_FORMAT: &[time::format_description::FormatItem] =
    format_description!("[year]-[month]-[day]");

// "%Y%m%d%H%M.%S" 15 chars
const YYYYMMDDHHMM_DOT_SS_FORMAT: &[time::format_description::FormatItem] = format_description!(
    "[year repr:full][month repr:numerical padding:zero]\
    [day][hour][minute].[second]"
);

// "%Y-%m-%d %H:%M:%S.%SS" 12 chars
const YYYYMMDDHHMMSS_FORMAT: &[time::format_description::FormatItem] = format_description!(
    "[year repr:full]-[month repr:numerical padding:zero]-\
    [day] [hour]:[minute]:[second].[subsecond]"
);

// "%Y-%m-%d %H:%M:%S" 12 chars
const YYYYMMDDHHMMS_FORMAT: &[time::format_description::FormatItem] = format_description!(
    "[year repr:full]-[month repr:numerical padding:zero]-\
    [day] [hour]:[minute]:[second]"
);

// "%Y-%m-%d %H:%M" 12 chars
// Used for example in tests/touch/no-rights.sh
const YYYY_MM_DD_HH_MM_FORMAT: &[time::format_description::FormatItem] = format_description!(
    "[year repr:full]-[month repr:numerical padding:zero]-\
    [day] [hour]:[minute]"
);

// "%Y%m%d%H%M" 12 chars
const YYYYMMDDHHMM_FORMAT: &[time::format_description::FormatItem] = format_description!(
    "[year repr:full][month repr:numerical padding:zero]\
    [day][hour][minute]"
);

// "%y%m%d%H%M.%S" 13 chars
const YYMMDDHHMM_DOT_SS_FORMAT: &[time::format_description::FormatItem] = format_description!(
    "[year repr:last_two padding:none][month][day]\
    [hour][minute].[second]"
);

// "%y%m%d%H%M" 10 chars
const YYMMDDHHMM_FORMAT: &[time::format_description::FormatItem] = format_description!(
    "[year repr:last_two padding:none][month padding:zero][day padding:zero]\
    [hour repr:24 padding:zero][minute padding:zero]"
);

// "%Y-%m-%d %H:%M +offset"
// Used for example in tests/touch/relative.sh
const YYYYMMDDHHMM_OFFSET_FORMAT: &[time::format_description::FormatItem] = format_description!(
    "[year]-[month]-[day] [hour repr:24]:[minute] \
    [offset_hour sign:mandatory][offset_minute]"
);

fn parse_date(s: &str) -> UResult<FileTime> {
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
    if let Ok(parsed) = time::PrimitiveDateTime::parse(s, &POSIX_LOCALE_FORMAT) {
        return Ok(local_dt_to_filetime(to_local(parsed)));
    }

    // Also support other formats found in the GNU tests like
    // in tests/misc/stat-nanoseconds.sh
    // or tests/touch/no-rights.sh
    for fmt in [
        YYYYMMDDHHMMS_FORMAT,
        YYYYMMDDHHMMSS_FORMAT,
        YYYY_MM_DD_HH_MM_FORMAT,
        YYYYMMDDHHMM_OFFSET_FORMAT,
    ] {
        if let Ok(parsed) = time::PrimitiveDateTime::parse(s, &fmt) {
            return Ok(dt_to_filename(parsed));
        }
    }

    // "Equivalent to %Y-%m-%d (the ISO 8601 date format). (C99)"
    // ("%F", ISO_8601_FORMAT),
    if let Ok(parsed) = time::Date::parse(s, &ISO_8601_FORMAT) {
        return Ok(local_dt_to_filetime(to_local(
            time::PrimitiveDateTime::new(parsed, time!(00:00)),
        )));
    }

    // "@%s" is "The number of seconds since the Epoch, 1970-01-01 00:00:00 +0000 (UTC). (TZ) (Calculated from mktime(tm).)"
    if s.bytes().next() == Some(b'@') {
        if let Ok(ts) = &s[1..].parse::<i64>() {
            // Don't convert to local time in this case - seconds since epoch are not time-zone dependent
            return Ok(local_dt_to_filetime(
                time::OffsetDateTime::from_unix_timestamp(*ts).unwrap(),
            ));
        }
    }

    if let Ok(duration) = parse_datetime::from_str(s) {
        let now_local = time::OffsetDateTime::now_local().unwrap();
        let diff = now_local
            .checked_add(time::Duration::nanoseconds(
                duration.num_nanoseconds().unwrap(),
            ))
            .unwrap();
        return Ok(local_dt_to_filetime(diff));
    }

    Err(USimpleError::new(1, format!("Unable to parse date: {s}")))
}

fn parse_timestamp(s: &str) -> UResult<FileTime> {
    // TODO: handle error
    let now = time::OffsetDateTime::now_utc();

    let (mut format, mut ts) = match s.chars().count() {
        15 => (YYYYMMDDHHMM_DOT_SS_FORMAT, s.to_owned()),
        12 => (YYYYMMDDHHMM_FORMAT, s.to_owned()),
        13 => (YYMMDDHHMM_DOT_SS_FORMAT, s.to_owned()),
        10 => (YYMMDDHHMM_FORMAT, s.to_owned()),
        11 => (YYYYMMDDHHMM_DOT_SS_FORMAT, format!("{}{}", now.year(), s)),
        8 => (YYYYMMDDHHMM_FORMAT, format!("{}{}", now.year(), s)),
        _ => {
            return Err(USimpleError::new(
                1,
                format!("invalid date format {}", s.quote()),
            ))
        }
    };
    // workaround time returning Err(TryFromParsed(InsufficientInformation)) for year w/
    // repr:last_two
    // https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=1ccfac7c07c5d1c7887a11decf0e1996
    if s.chars().count() == 10 {
        format = YYYYMMDDHHMM_FORMAT;
        ts = "20".to_owned() + &ts;
    } else if s.chars().count() == 13 {
        format = YYYYMMDDHHMM_DOT_SS_FORMAT;
        ts = "20".to_owned() + &ts;
    }

    let leap_sec = if (format == YYYYMMDDHHMM_DOT_SS_FORMAT || format == YYMMDDHHMM_DOT_SS_FORMAT)
        && ts.ends_with(".60")
    {
        // Work around to disable leap seconds
        // Used in gnu/tests/touch/60-seconds
        ts = ts.replace(".60", ".59");
        true
    } else {
        false
    };

    let tm = time::PrimitiveDateTime::parse(&ts, &format)
        .map_err(|_| USimpleError::new(1, format!("invalid date ts format {}", ts.quote())))?;
    let mut local = to_local(tm);
    if leap_sec {
        // We are dealing with a leap second, add it
        local = local.saturating_add(Duration::SECOND);
    }
    let ft = local_dt_to_filetime(local);

    // // We have to check that ft is valid time. Due to daylight saving
    // // time switch, local time can jump from 1:59 AM to 3:00 AM,
    // // in which case any time between 2:00 AM and 2:59 AM is not valid.
    // // Convert back to local time and see if we got the same value back.
    // let ts = time::Timespec {
    //     sec: ft.unix_seconds(),
    //     nsec: 0,
    // };
    // let tm2 = time::at(ts);
    // if tm.tm_hour != tm2.tm_hour {
    //     return Err(USimpleError::new(
    //         1,
    //         format!("invalid date format {}", s.quote()),
    //     ));
    // }

    Ok(ft)
}

// TODO: this may be a good candidate to put in fsext.rs
/// Returns a PathBuf to stdout.
///
/// On Windows, uses GetFinalPathNameByHandleW to attempt to get the path
/// from the stdout handle.
fn pathbuf_from_stdout() -> UResult<PathBuf> {
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
            GetLastError, ERROR_INVALID_PARAMETER, ERROR_NOT_ENOUGH_MEMORY, ERROR_PATH_NOT_FOUND,
            HANDLE, MAX_PATH,
        };
        use windows_sys::Win32::Storage::FileSystem::{
            GetFinalPathNameByHandleW, FILE_NAME_OPENED,
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
                return Err(USimpleError::new(
                    1,
                    format!("GetFinalPathNameByHandleW failed with code {ret}"),
                ))
            }
            e if e == 0 => {
                return Err(USimpleError::new(
                    1,
                    format!(
                        "GetFinalPathNameByHandleW failed with code {}",
                        // SAFETY: GetLastError is thread-safe and has no documented memory unsafety.
                        unsafe { GetLastError() }
                    ),
                ));
            }
            e => e as usize,
        };

        // Don't include the null terminator
        Ok(String::from_utf16(&file_path_buffer[0..buffer_size])
            .map_err(|e| USimpleError::new(1, e.to_string()))?
            .into())
    }
}

#[cfg(test)]
mod tests {
    #[cfg(windows)]
    #[test]
    fn test_get_pathbuf_from_stdout_fails_if_stdout_is_not_a_file() {
        // We can trigger an error by not setting stdout to anything (will
        // fail with code 1)
        assert!(super::pathbuf_from_stdout()
            .expect_err("pathbuf_from_stdout should have failed")
            .to_string()
            .contains("GetFinalPathNameByHandleW failed with code 1"));
    }
}
