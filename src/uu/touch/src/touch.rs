// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) filetime datetime lpszfilepath mktime DATETIME datelike timelike
// spell-checker:ignore (FORMATS) MMDDhhmm YYYYMMDDHHMM YYMMDDHHMM YYYYMMDDHHMMS

use chrono::{
    DateTime, Datelike, Duration, Local, LocalResult, NaiveDate, NaiveDateTime, NaiveTime,
    TimeZone, Timelike,
};
use clap::ArgMatches;
use filetime::{set_file_times, set_symlink_file_times, FileTime};
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{Error, ErrorKind};
use std::path::{Path, PathBuf};
use uucore::display::Quotable;
use uucore::error::{FromIo, UResult, USimpleError};
use uucore::show;

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

/// Convert a DateTime with a TZ offset into a FileTime
///
/// The DateTime is converted into a unix timestamp from which the FileTime is
/// constructed.
fn datetime_to_filetime<T: TimeZone>(dt: &DateTime<T>) -> FileTime {
    FileTime::from_unix_time(dt.timestamp(), dt.timestamp_subsec_nanos())
}

fn filetime_to_datetime(ft: &FileTime) -> Option<DateTime<Local>> {
    Some(DateTime::from_timestamp(ft.unix_seconds(), ft.nanoseconds())?.into())
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = crate::uu_app().try_get_matches_from(args)?;

    let files = matches
        .get_many::<OsString>(crate::options::ARG_FILES)
        .ok_or_else(|| {
            USimpleError::new(
                1,
                format!(
                    "missing file operand\nTry '{} --help' for more information.",
                    uucore::execution_phrase()
                ),
            )
        })?;

    let (atime, mtime) = determine_times(&matches)?;

    for filename in files {
        // FIXME: find a way to avoid having to clone the path
        let pathbuf = if filename == "-" {
            pathbuf_from_stdout()?
        } else {
            PathBuf::from(filename)
        };

        let path = pathbuf.as_path();

        let metadata_result = if matches.get_flag(crate::options::NO_DEREF) {
            path.symlink_metadata()
        } else {
            path.metadata()
        };

        if let Err(e) = metadata_result {
            if e.kind() != std::io::ErrorKind::NotFound {
                return Err(e.map_err_context(|| format!("setting times of {}", filename.quote())));
            }

            if matches.get_flag(crate::options::NO_CREATE) {
                continue;
            }

            if matches.get_flag(crate::options::NO_DEREF) {
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
                    let custom_err = Error::new(ErrorKind::Other, "No such file or directory");
                    return Err(
                        custom_err.map_err_context(|| format!("cannot touch {}", filename.quote()))
                    );
                } else {
                    show!(e.map_err_context(|| format!("cannot touch {}", path.quote())));
                }
                continue;
            };

            // Minor optimization: if no reference time was specified, we're done.
            if !matches.contains_id(crate::options::SOURCES) {
                continue;
            }
        }

        update_times(&matches, path, atime, mtime, filename)?;
    }
    Ok(())
}

// Determine the access and modification time
fn determine_times(matches: &ArgMatches) -> UResult<(FileTime, FileTime)> {
    match (
        matches.get_one::<OsString>(crate::options::sources::REFERENCE),
        matches.get_one::<String>(crate::options::sources::DATE),
    ) {
        (Some(reference), Some(date)) => {
            let (atime, mtime) = stat(
                Path::new(&reference),
                !matches.get_flag(crate::options::NO_DEREF),
            )?;
            let atime = filetime_to_datetime(&atime).ok_or_else(|| {
                USimpleError::new(1, "Could not process the reference access time")
            })?;
            let mtime = filetime_to_datetime(&mtime).ok_or_else(|| {
                USimpleError::new(1, "Could not process the reference modification time")
            })?;
            Ok((parse_date(atime, date)?, parse_date(mtime, date)?))
        }
        (Some(reference), None) => stat(
            Path::new(&reference),
            !matches.get_flag(crate::options::NO_DEREF),
        ),
        (None, Some(date)) => {
            let timestamp = parse_date(Local::now(), date)?;
            Ok((timestamp, timestamp))
        }
        (None, None) => {
            let timestamp =
                if let Some(ts) = matches.get_one::<String>(crate::options::sources::TIMESTAMP) {
                    parse_timestamp(ts)?
                } else {
                    datetime_to_filetime(&Local::now())
                };
            Ok((timestamp, timestamp))
        }
    }
}

// Updating file access and modification times based on user-specified options
fn update_times(
    matches: &ArgMatches,
    path: &Path,
    mut atime: FileTime,
    mut mtime: FileTime,
    filename: &OsString,
) -> UResult<()> {
    // If changing "only" atime or mtime, grab the existing value of the other.
    // Note that "-a" and "-m" may be passed together; this is not an xor.
    if matches.get_flag(crate::options::ACCESS)
        || matches.get_flag(crate::options::MODIFICATION)
        || matches.contains_id(crate::options::TIME)
    {
        let st = stat(path, !matches.get_flag(crate::options::NO_DEREF))?;
        let time = matches
            .get_one::<String>(crate::options::TIME)
            .map(|s| s.as_str())
            .unwrap_or("");

        if !(matches.get_flag(crate::options::ACCESS)
            || time.contains(&"access".to_owned())
            || time.contains(&"atime".to_owned())
            || time.contains(&"use".to_owned()))
        {
            atime = st.0;
        }

        if !(matches.get_flag(crate::options::MODIFICATION)
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
    } else if matches.get_flag(crate::options::NO_DEREF) {
        set_symlink_file_times(path, atime, mtime)
    } else {
        set_file_times(path, atime, mtime)
    }
    .map_err_context(|| format!("setting times of {}", path.quote()))
}
// Get metadata of the provided path
// If `follow` is `true`, the function will try to follow symlinks
// If `follow` is `false` or the symlink is broken, the function will return metadata of the symlink itself
fn stat(path: &Path, follow: bool) -> UResult<(FileTime, FileTime)> {
    let metadata = if follow {
        fs::metadata(path).or_else(|_| fs::symlink_metadata(path))
    } else {
        fs::symlink_metadata(path)
    }
    .map_err_context(|| format!("failed to get attributes of {}", path.quote()))?;

    Ok((
        FileTime::from_last_access_time(&metadata),
        FileTime::from_last_modification_time(&metadata),
    ))
}

fn parse_date(ref_time: DateTime<Local>, s: &str) -> UResult<FileTime> {
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

    if let Ok(dt) = parse_datetime::parse_datetime_at_date(ref_time, s) {
        return Ok(datetime_to_filetime(&dt));
    }

    Err(USimpleError::new(1, format!("Unable to parse date: {s}")))
}

fn parse_timestamp(s: &str) -> UResult<FileTime> {
    use format::*;

    let current_year = || Local::now().year();

    let (format, ts) = match s.chars().count() {
        15 => (YYYYMMDDHHMM_DOT_SS, s.to_owned()),
        12 => (YYYYMMDDHHMM, s.to_owned()),
        // If we don't add "20", we have insufficient information to parse
        13 => (YYYYMMDDHHMM_DOT_SS, format!("20{}", s)),
        10 => (YYYYMMDDHHMM, format!("20{}", s)),
        11 => (YYYYMMDDHHMM_DOT_SS, format!("{}{}", current_year(), s)),
        8 => (YYYYMMDDHHMM, format!("{}{}", current_year(), s)),
        _ => {
            return Err(USimpleError::new(
                1,
                format!("invalid date format {}", s.quote()),
            ))
        }
    };

    let local = NaiveDateTime::parse_from_str(&ts, format)
        .map_err(|_| USimpleError::new(1, format!("invalid date ts format {}", ts.quote())))?;
    let mut local = match chrono::Local.from_local_datetime(&local) {
        LocalResult::Single(dt) => dt,
        _ => {
            return Err(USimpleError::new(
                1,
                format!("invalid date ts format {}", ts.quote()),
            ))
        }
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
            format!("invalid date format {}", s.quote()),
        ));
    }

    Ok(datetime_to_filetime(&local))
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
            0 => {
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
