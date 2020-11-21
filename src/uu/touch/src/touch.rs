// This file is part of the uutils coreutils package.
//
// (c) Nick Platt <platt.nicholas@gmail.com>
// (c) Jian Zeng <anonymousknight96 AT gmail.com>
//
// For the full copyright and license information, please view the LICENSE file
// that was distributed with this source code.

// spell-checker:ignore (ToDO) filetime strptime utcoff strs datetime MMDDhhmm

extern crate clap;
pub extern crate filetime;
extern crate time;

#[macro_use]
extern crate uucore;

use clap::{App, Arg};
use filetime::*;
use std::fs::{self, File};
use std::io::Error;
use std::path::Path;

static VERSION: &str = env!("CARGO_PKG_VERSION");
static ABOUT: &str = "Update the access and modification times of each FILE to the current time.";
static OPT_ACCESS: &str = "access";
static OPT_CURRENT: &str = "current";
static OPT_DATE: &str = "date";
static OPT_MODIFICATION: &str = "modification";
static OPT_NO_CREATE: &str = "no-create";
static OPT_NO_DEREF: &str = "no-dereference";
static OPT_REFERENCE: &str = "reference";
static OPT_TIME: &str = "time";

static ARG_FILES: &str = "files";

// Since touch's date/timestamp parsing doesn't account for timezone, the
// returned value from time::strptime() is UTC. We get system's timezone to
// localize the time.
macro_rules! to_local(
    ($exp:expr) => ({
        let mut tm = $exp;
        tm.tm_utcoff = time::now().tm_utcoff;
        tm
    })
);

macro_rules! local_tm_to_filetime(
    ($exp:expr) => ({
        let ts = $exp.to_timespec();
        FileTime::from_unix_time(ts.sec as i64, ts.nsec as u32)
    })
);

fn get_usage() -> String {
    format!("{0} [OPTION]... [USER]", executable!())
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let usage = get_usage();

    let matches = App::new(executable!())
        .version(VERSION)
        .about(ABOUT)
        .usage(&usage[..])
        .arg(
            Arg::with_name(OPT_ACCESS)
                .short("a")
                .help("change only the access time"),
        )
        .arg(
            Arg::with_name(OPT_CURRENT)
                .short("t")
                .help("use [[CC]YY]MMDDhhmm[.ss] instead of the current time")
                .value_name("STAMP")
                .takes_value(true),
        )
        .arg(
            Arg::with_name(OPT_DATE)
                .short("d")
                .long(OPT_DATE)
                .help("parse argument and use it instead of current time")
                .value_name("STRING"),
        )
        .arg(
            Arg::with_name(OPT_MODIFICATION)
                .short("m")
                .help("change only the modification time"),
        )
        .arg(
            Arg::with_name(OPT_NO_CREATE)
                .short("c")
                .long(OPT_NO_CREATE)
                .help("do not create any files"),
        )
        .arg(
            Arg::with_name(OPT_NO_DEREF)
                .short("h")
                .long(OPT_NO_DEREF)
                .help(
                    "affect each symbolic link instead of any referenced file \
                     (only for systems that can change the timestamps of a symlink)",
                ),
        )
        .arg(
            Arg::with_name(OPT_REFERENCE)
                .short("r")
                .long(OPT_REFERENCE)
                .help("use this file's times instead of the current time")
                .value_name("FILE"),
        )
        .arg(
            Arg::with_name(OPT_TIME)
                .long(OPT_TIME)
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
            Arg::with_name(ARG_FILES)
                .multiple(true)
                .takes_value(true)
                .min_values(1),
        )
        .get_matches_from(args);

    let files: Vec<String> = matches
        .values_of(ARG_FILES)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_default();

    if matches.is_present(OPT_DATE)
        && (matches.is_present(OPT_REFERENCE) || matches.is_present(OPT_CURRENT))
        || matches.is_present(OPT_REFERENCE)
            && (matches.is_present(OPT_DATE) || matches.is_present(OPT_CURRENT))
        || matches.is_present(OPT_CURRENT)
            && (matches.is_present(OPT_DATE) || matches.is_present(OPT_REFERENCE))
    {
        panic!("Invalid options: cannot specify reference time from more than one source");
    }

    let (mut atime, mut mtime) = if matches.is_present(OPT_REFERENCE) {
        stat(
            &matches.value_of(OPT_REFERENCE).unwrap()[..],
            !matches.is_present(OPT_NO_DEREF),
        )
    } else if matches.is_present(OPT_DATE) || matches.is_present(OPT_CURRENT) {
        let timestamp = if matches.is_present(OPT_DATE) {
            parse_date(matches.value_of(OPT_DATE).unwrap().as_ref())
        } else {
            parse_timestamp(matches.value_of(OPT_CURRENT).unwrap().as_ref())
        };
        (timestamp, timestamp)
    } else {
        let now = local_tm_to_filetime!(time::now());
        (now, now)
    };

    for filename in &files {
        let path = &filename[..];

        if !Path::new(path).exists() {
            // no-dereference included here for compatibility
            if matches.is_present(OPT_NO_CREATE) || matches.is_present(OPT_NO_DEREF) {
                continue;
            }

            if let Err(e) = File::create(path) {
                show_warning!("cannot touch '{}': {}", path, e);
                continue;
            };

            // Minor optimization: if no reference time was specified, we're done.
            if !(matches.is_present(OPT_DATE)
                || matches.is_present(OPT_REFERENCE)
                || matches.is_present(OPT_CURRENT))
            {
                continue;
            }
        }

        // If changing "only" atime or mtime, grab the existing value of the other.
        // Note that "-a" and "-m" may be passed together; this is not an xor.
        if matches.is_present(OPT_ACCESS)
            || matches.is_present(OPT_MODIFICATION)
            || matches.is_present(OPT_TIME)
        {
            let st = stat(path, !matches.is_present(OPT_NO_DEREF));
            let time = matches.value_of(OPT_TIME).unwrap_or("");

            if !(matches.is_present(OPT_ACCESS)
                || time.contains(&"access".to_owned())
                || time.contains(&"atime".to_owned())
                || time.contains(&"use".to_owned()))
            {
                atime = st.0;
            }

            if !(matches.is_present(OPT_MODIFICATION)
                || time.contains(&"modify".to_owned())
                || time.contains(&"mtime".to_owned()))
            {
                mtime = st.1;
            }
        }

        if matches.is_present(OPT_NO_DEREF) {
            if let Err(e) = set_symlink_file_times(path, atime, mtime) {
                show_warning!("cannot touch '{}': {}", path, e);
            }
        } else if let Err(e) = filetime::set_file_times(path, atime, mtime) {
            show_warning!("cannot touch '{}': {}", path, e);
        }
    }

    0
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
    match time::strptime(str, "%c") {
        Ok(tm) => local_tm_to_filetime!(to_local!(tm)),
        Err(e) => panic!("Unable to parse date\n{}", e),
    }
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
        Ok(tm) => local_tm_to_filetime!(to_local!(tm)),
        Err(e) => panic!("Unable to parse timestamp\n{}", e),
    }
}
