// This file is part of the uutils coreutils package.
//
// (c) Nick Platt <platt.nicholas@gmail.com>
// (c) Jian Zeng <anonymousknight96 AT gmail.com>
//
// For the full copyright and license information, please view the LICENSE file
// that was distributed with this source code.

// spell-checker:ignore (ToDO) filetime strptime utcoff strs datetime

pub extern crate filetime;

#[macro_use]
extern crate uucore;

use filetime::*;
use std::fs::{self, File};
use std::io::Error;
use std::path::Path;
use std::process;

use crate::app::{get_app, options, ARG_FILES};

mod app;

fn to_local(mut tm: time::Tm) -> time::Tm {
    tm.tm_utcoff = time::now().tm_utcoff;
    tm
}

fn local_tm_to_filetime(tm: time::Tm) -> FileTime {
    let ts = tm.to_timespec();
    FileTime::from_unix_time(ts.sec as i64, ts.nsec as u32)
}

fn get_usage() -> String {
    format!("{0} [OPTION]... [USER]", executable!())
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let usage = get_usage();

    let matches = get_app(executable!())
        .usage(&usage[..])
        .get_matches_from(args);

    let files: Vec<String> = matches
        .values_of(ARG_FILES)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_default();

    let (mut atime, mut mtime) = if matches.is_present(options::sources::REFERENCE) {
        stat(
            matches.value_of(options::sources::REFERENCE).unwrap(),
            !matches.is_present(options::NO_DEREF),
        )
    } else if matches.is_present(options::sources::DATE)
        || matches.is_present(options::sources::CURRENT)
    {
        let timestamp = if matches.is_present(options::sources::DATE) {
            parse_date(matches.value_of(options::sources::DATE).unwrap())
        } else {
            parse_timestamp(matches.value_of(options::sources::CURRENT).unwrap())
        };
        (timestamp, timestamp)
    } else {
        let now = local_tm_to_filetime(time::now());
        (now, now)
    };

    let mut error_code = 0;

    for filename in &files {
        let path = &filename[..];

        if !Path::new(path).exists() {
            // no-dereference included here for compatibility
            if matches.is_present(options::NO_CREATE) || matches.is_present(options::NO_DEREF) {
                continue;
            }

            if let Err(e) = File::create(path) {
                show_warning!("cannot touch '{}': {}", path, e);
                error_code = 1;
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
            let st = stat(path, !matches.is_present(options::NO_DEREF));
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
            if let Err(e) = set_symlink_file_times(path, atime, mtime) {
                // we found an error, it should fail in any case
                error_code = 1;
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    // GNU compatibility (not-owner.sh)
                    show_error!("setting times of '{}': {}", path, "Permission denied");
                } else {
                    show_error!("setting times of '{}': {}", path, e);
                }
            }
        } else if let Err(e) = filetime::set_file_times(path, atime, mtime) {
            // we found an error, it should fail in any case
            error_code = 1;

            if e.kind() == std::io::ErrorKind::PermissionDenied {
                // GNU compatibility (not-owner.sh)
                show_error!("setting times of '{}': {}", path, "Permission denied");
            } else {
                show_error!("setting times of '{}': {}", path, e);
            }
        }
    }
    error_code
}

fn stat(path: &str, follow: bool) -> (FileTime, FileTime) {
    let metadata = if follow {
        fs::symlink_metadata(path)
    } else {
        fs::metadata(path)
    };

    match metadata {
        Ok(m) => (
            FileTime::from_last_access_time(&m),
            FileTime::from_last_modification_time(&m),
        ),
        Err(_) => crash!(
            1,
            "failed to get attributes of '{}': {}",
            path,
            Error::last_os_error()
        ),
    }
}

fn parse_date(str: &str) -> FileTime {
    // This isn't actually compatible with GNU touch, but there doesn't seem to
    // be any simple specification for what format this parameter allows and I'm
    // not about to implement GNU parse_datetime.
    // http://git.savannah.gnu.org/gitweb/?p=gnulib.git;a=blob_plain;f=lib/parse-datetime.y
    let formats = vec!["%c", "%F"];
    for f in formats {
        if let Ok(tm) = time::strptime(str, f) {
            return local_tm_to_filetime(to_local(tm));
        }
    }
    show_error!("Unable to parse date: {}\n", str);
    process::exit(1);
}

fn parse_timestamp(s: &str) -> FileTime {
    let now = time::now();
    let (format, ts) = match s.chars().count() {
        15 => ("%Y%m%d%H%M.%S", s.to_owned()),
        12 => ("%Y%m%d%H%M", s.to_owned()),
        13 => ("%y%m%d%H%M.%S", s.to_owned()),
        10 => ("%y%m%d%H%M", s.to_owned()),
        11 => ("%Y%m%d%H%M.%S", format!("{}{}", now.tm_year + 1900, s)),
        8 => ("%Y%m%d%H%M", format!("{}{}", now.tm_year + 1900, s)),
        _ => panic!("Unknown timestamp format"),
    };

    match time::strptime(&ts, format) {
        Ok(tm) => {
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
                show_error!("invalid date format {}", s);
                process::exit(1);
            }

            ft
        }
        Err(e) => panic!("Unable to parse timestamp\n{}", e),
    }
}
