// This file is part of the uutils coreutils package.
//
// (c) Nick Platt <platt.nicholas@gmail.com>
// (c) Jian Zeng <anonymousknight96 AT gmail.com>
//
// For the full copyright and license information, please view the LICENSE file
// that was distributed with this source code.

// spell-checker:ignore (ToDO) filetime strptime utcoff strs datetime MMDDhhmm clapv

pub extern crate filetime;

#[macro_use]
extern crate uucore;

use clap::{crate_version, App, AppSettings, Arg, ArgGroup};
use filetime::*;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use uucore::display::Quotable;
use uucore::error::{FromIo, UError, UResult, USimpleError};

static ABOUT: &str = "Update the access and modification times of each FILE to the current time.";
pub mod options {
    // Both SOURCES and sources are needed as we need to be able to refer to the ArgGroup.
    pub static SOURCES: &str = "sources";
    pub mod sources {
        pub static DATE: &str = "date";
        pub static REFERENCE: &str = "reference";
        pub static CURRENT: &str = "current";
    }
    pub static ACCESS: &str = "access";
    pub static MODIFICATION: &str = "modification";
    pub static NO_CREATE: &str = "no-create";
    pub static NO_DEREF: &str = "no-dereference";
    pub static TIME: &str = "time";
}

static ARG_FILES: &str = "files";

fn to_local(mut tm: time::Tm) -> time::Tm {
    tm.tm_utcoff = time::now().tm_utcoff;
    tm
}

fn local_tm_to_filetime(tm: time::Tm) -> FileTime {
    let ts = tm.to_timespec();
    FileTime::from_unix_time(ts.sec as i64, ts.nsec as u32)
}

fn usage() -> String {
    format!("{0} [OPTION]... [USER]", uucore::execution_phrase())
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let usage = usage();

    let matches = uu_app().override_usage(&usage[..]).get_matches_from(args);

    let files = matches.values_of_os(ARG_FILES).ok_or_else(|| {
        USimpleError::new(
            1,
            r##"missing file operand
Try 'touch --help' for more information."##,
        )
    })?;

    let (mut atime, mut mtime) =
        if let Some(reference) = matches.value_of_os(options::sources::REFERENCE) {
            stat(Path::new(reference), !matches.is_present(options::NO_DEREF))?
        } else {
            let timestamp = if let Some(date) = matches.value_of(options::sources::DATE) {
                parse_date(date)?
            } else if let Some(current) = matches.value_of(options::sources::CURRENT) {
                parse_timestamp(current)?
            } else {
                local_tm_to_filetime(time::now())
            };
            (timestamp, timestamp)
        };

    for filename in files {
        // FIXME: find a way to avoid having to clone the path
        let pathbuf = if filename == "-" {
            path_from_stdout()?
        } else {
            PathBuf::from(filename)
        };

        let path = pathbuf.as_path();

        if !path.exists() {
            if matches.is_present(options::NO_CREATE) {
                continue;
            }

            if matches.is_present(options::NO_DEREF) {
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
            if !matches.is_present(options::SOURCES) {
                continue;
            }
        }

        // If changing "only" atime or mtime, grab the existing value of the other.
        // Note that "-a" and "-m" may be passed together; this is not an xor.
        if matches.is_present(options::ACCESS)
            || matches.is_present(options::MODIFICATION)
            || matches.is_present(options::TIME)
        {
            let st = stat(path, !matches.is_present(options::NO_DEREF))?;
            let time = matches.value_of(options::TIME).unwrap_or("");

            if !(matches.is_present(options::ACCESS)
                || time.contains(&"access".to_owned())
                || time.contains(&"atime".to_owned())
                || time.contains(&"use".to_owned()))
            {
                atime = st.0;
            }

            if !(matches.is_present(options::MODIFICATION)
                || time.contains(&"modify".to_owned())
                || time.contains(&"mtime".to_owned()))
            {
                mtime = st.1;
            }
        }

        if matches.is_present(options::NO_DEREF) {
            set_symlink_file_times(path, atime, mtime)
        } else {
            filetime::set_file_times(path, atime, mtime)
        }
        .map_err_context(|| format!("setting times of {}", path.quote()))?;
    }

    Ok(())
}

pub fn uu_app<'a>() -> App<'a> {
    App::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .setting(AppSettings::InferLongArgs)
        .arg(
            Arg::new(options::ACCESS)
                .short('a')
                .help("change only the access time"),
        )
        .arg(
            Arg::new(options::sources::CURRENT)
                .short('t')
                .help("use [[CC]YY]MMDDhhmm[.ss] instead of the current time")
                .value_name("STAMP")
                .takes_value(true),
        )
        .arg(
            Arg::new(options::sources::DATE)
                .short('d')
                .long(options::sources::DATE)
                .help("parse argument and use it instead of current time")
                .value_name("STRING"),
        )
        .arg(
            Arg::new(options::MODIFICATION)
                .short('m')
                .help("change only the modification time"),
        )
        .arg(
            Arg::new(options::NO_CREATE)
                .short('c')
                .long(options::NO_CREATE)
                .help("do not create any files"),
        )
        .arg(
            Arg::new(options::NO_DEREF)
                .short('h')
                .long(options::NO_DEREF)
                .help(
                    "affect each symbolic link instead of any referenced file \
                     (only for systems that can change the timestamps of a symlink)",
                ),
        )
        .arg(
            Arg::new(options::sources::REFERENCE)
                .short('r')
                .long(options::sources::REFERENCE)
                .help("use this file's times instead of the current time")
                .value_name("FILE")
                .allow_invalid_utf8(true),
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
                .possible_values(&["access", "atime", "use"])
                .takes_value(true),
        )
        .arg(
            Arg::new(ARG_FILES)
                .multiple_occurrences(true)
                .takes_value(true)
                .min_values(1)
                .allow_invalid_utf8(true),
        )
        .group(ArgGroup::new(options::SOURCES).args(&[
            options::sources::CURRENT,
            options::sources::DATE,
            options::sources::REFERENCE,
        ]))
}

fn stat(path: &Path, follow: bool) -> UResult<(FileTime, FileTime)> {
    let metadata = if follow {
        fs::symlink_metadata(path)
    } else {
        fs::metadata(path)
    }
    .map_err_context(|| format!("failed to get attributes of {}", path.quote()))?;

    Ok((
        FileTime::from_last_access_time(&metadata),
        FileTime::from_last_modification_time(&metadata),
    ))
}

fn parse_date(str: &str) -> UResult<FileTime> {
    // This isn't actually compatible with GNU touch, but there doesn't seem to
    // be any simple specification for what format this parameter allows and I'm
    // not about to implement GNU parse_datetime.
    // http://git.savannah.gnu.org/gitweb/?p=gnulib.git;a=blob_plain;f=lib/parse-datetime.y
    let formats = vec!["%c", "%F"];
    for f in formats {
        if let Ok(tm) = time::strptime(str, f) {
            return Ok(local_tm_to_filetime(to_local(tm)));
        }
    }

    if let Ok(tm) = time::strptime(str, "@%s") {
        // Don't convert to local time in this case - seconds since epoch are not time-zone dependent
        return Ok(local_tm_to_filetime(tm));
    }

    Err(USimpleError::new(
        1,
        format!("Unable to parse date: {}", str),
    ))
}

fn parse_timestamp(s: &str) -> UResult<FileTime> {
    let now = time::now();
    let (format, ts) = match s.chars().count() {
        15 => ("%Y%m%d%H%M.%S", s.to_owned()),
        12 => ("%Y%m%d%H%M", s.to_owned()),
        13 => ("%y%m%d%H%M.%S", s.to_owned()),
        10 => ("%y%m%d%H%M", s.to_owned()),
        11 => ("%Y%m%d%H%M.%S", format!("{}{}", now.tm_year + 1900, s)),
        8 => ("%Y%m%d%H%M", format!("{}{}", now.tm_year + 1900, s)),
        _ => {
            return Err(USimpleError::new(
                1,
                format!("invalid date format {}", s.quote()),
            ))
        }
    };

    let tm = time::strptime(&ts, format)
        .map_err(|_| USimpleError::new(1, format!("invalid date format {}", s.quote())))?;

    let mut local = to_local(tm);
    local.tm_isdst = -1;
    let ft = local_tm_to_filetime(local);

    // We have to check that ft is valid time. Due to daylight saving
    // time switch, local time can jump from 1:59 AM to 3:00 AM,
    // in which case any time between 2:00 AM and 2:59 AM is not valid.
    // Convert back to local time and see if we got the same value back.
    let ts = time::Timespec {
        sec: ft.unix_seconds(),
        nsec: 0,
    };
    let tm2 = time::at(ts);
    if tm.tm_hour != tm2.tm_hour {
        return Err(USimpleError::new(
            1,
            format!("invalid date format {}", s.quote()),
        ));
    }

    Ok(ft)
}

/// Returns a PathBuf to stdout.
///
/// On Windows, uses GetFinalPathNameByHandleW to attempt to get the path
/// from the stdout handle.
fn path_from_stdout() -> UResult<PathBuf> {
    #[cfg(unix)]
    {
        PathBuf::from("/dev/stdout")
    }
    #[cfg(windows)]
    {
        use std::os::windows::prelude::AsRawHandle;
        use windows::Win32::Foundation::{
            GetLastError, ERROR_INVALID_PARAMETER, ERROR_NOT_ENOUGH_MEMORY, ERROR_PATH_NOT_FOUND,
            HANDLE, MAX_PATH, PWSTR, WIN32_ERROR,
        };
        use windows::Win32::Storage::FileSystem::FILE_NAME_OPENED;

        unsafe {
            let handle = std::io::stdout().lock().as_raw_handle();
            let file_path_buffer = [0u16; MAX_PATH as usize];

            // https://docs.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getfinalpathnamebyhandlea#examples
            let ret = windows::Win32::Storage::FileSystem::GetFinalPathNameByHandleW::<HANDLE>(
                std::mem::transmute(handle),
                PWSTR(file_path_buffer.as_ptr()),
                MAX_PATH,
                FILE_NAME_OPENED,
            );

            // TODO: there is probably a library or something for handling win32 errors
            // more elegantly
            let err = WIN32_ERROR(ret);
            let bufsize = match err {
                ERROR_PATH_NOT_FOUND => Err(USimpleError::new(
                    1,
                    "file pointed to by stdout does not exist (should never happen)",
                )),
                ERROR_NOT_ENOUGH_MEMORY => Err(USimpleError::new(
                    1,
                    "not enough memory to complete the operation",
                )),
                ERROR_INVALID_PARAMETER => Err(USimpleError::new(1, "Invalid dwFlags")),
                e if e.0 >= MAX_PATH => {
                    Err(USimpleError::new(1, format!("need buffer size of {}", e.0)))
                }
                e if e.0 == 0 => Err(USimpleError::new(
                    1,
                    format!("error code: {}", GetLastError().0),
                )),
                e => Ok(e.0 as usize),
            }?;

            // Don't include the null terminator
            Ok(String::from_utf16(&file_path_buffer[0..bufsize])
                .map_err(|e| USimpleError::new(1, e.to_string()))?
                .into())
        }
    }
}
