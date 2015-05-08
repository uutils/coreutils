#![crate_name = "touch"]
#![feature(rustc_private, path_ext, fs_time)]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Nick Platt <platt.nicholas@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE file
 * that was distributed with this source code.
 */

extern crate libc;
extern crate getopts;
extern crate time;

use libc::types::os::arch::c95::c_char;
use libc::types::os::arch::posix01::stat as stat_t;
use libc::funcs::posix88::stat_::stat as c_stat;
use libc::funcs::posix01::stat_::lstat as c_lstat;

use std::fs::{set_file_times, File, PathExt};
use std::io::{Error, Write};
use std::mem::uninitialized;
use std::path::Path;

#[path = "../common/util.rs"]
#[macro_use]
mod util;

static NAME: &'static str = "touch";
static VERSION: &'static str = "1.0.0";

pub fn uumain(args: Vec<String>) -> i32 {
    let opts = [
        getopts::optflag("a", "",               "change only the access time"),
        getopts::optflag("c", "no-create",      "do not create any files"),
        getopts::optopt( "d", "date",           "parse argument and use it instead of current time", "STRING"),
        getopts::optflag("h", "no-dereference", "affect each symbolic link instead of any referenced file \
                                                 (only for systems that can change the timestamps of a symlink)"),
        getopts::optflag("m", "",               "change only the modification time"),
        getopts::optopt( "r", "reference",      "use this file's times instead of the current time", "FILE"),
        getopts::optopt( "t", "",               "use [[CC]YY]MMDDhhmm[.ss] instead of the current time", "STAMP"),
        getopts::optopt( "",  "time",           "change only the specified time: \"access\", \"atime\", or \
                                                 \"use\" are equivalent to -a; \"modify\" or \"mtime\" are \
                                                 equivalent to -m", "WORD"),
        getopts::optflag("h", "help",           "display this help and exit"),
        getopts::optflag("V", "version",        "output version information and exit"),
    ];

    let matches = match getopts::getopts(&args[1..], &opts) {
        Ok(m)  => m,
        Err(e) => panic!("Invalid options\n{}", e)
    };

    if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
        return 0;
    }

    if matches.opt_present("help") || matches.free.is_empty() {
        println!("{} {}", NAME, VERSION);
        println!("");
        println!("Usage: {} [OPTION]... FILE...", NAME);
        println!("");
        println!("{}", getopts::usage("Update the access and modification times of \
                                         each FILE to the current time.", &opts));
        if matches.free.is_empty() {
            return 1;
        }
        return 0;
    }

    if matches.opt_present("date") && matches.opts_present(&["reference".to_string(), "t".to_string()]) ||
       matches.opt_present("reference") && matches.opts_present(&["date".to_string(), "t".to_string()]) ||
       matches.opt_present("t") && matches.opts_present(&["date".to_string(), "reference".to_string()]) {
        panic!("Invalid options: cannot specify reference time from more than one source");
    }

    let (mut atime, mut mtime) =
        if matches.opt_present("reference") {
            stat(&matches.opt_str("reference").unwrap()[..], !matches.opt_present("no-dereference"))
        } else if matches.opts_present(&["date".to_string(), "t".to_string()]) {
            let timestamp = if matches.opt_present("date") {
                parse_date(matches.opt_str("date").unwrap().as_ref())
            } else {
                parse_timestamp(matches.opt_str("t").unwrap().as_ref())
            };
            (timestamp, timestamp)
        } else {
            // FIXME: Should use Timespec. https://github.com/mozilla/rust/issues/10301
            let now = (time::get_time().sec * 1000) as u64;
            (now, now)
        };

    for filename in matches.free.iter() {
        let path = &filename[..];

        if ! Path::new(path).exists() {
            // no-dereference included here for compatibility
            if matches.opts_present(&["no-create".to_string(), "no-dereference".to_string()]) {
                continue;
            }

            match File::create(path) {
                Err(e) => {
                    show_warning!("cannot touch '{}': {}", path, e);
                    continue;
                },
                _ => (),
            };

            // Minor optimization: if no reference time was specified, we're done.
            if !matches.opts_present(&["date".to_string(), "reference".to_string(), "t".to_string()]) {
                continue;
            }
        }

        // If changing "only" atime or mtime, grab the existing value of the other.
        // Note that "-a" and "-m" may be passed together; this is not an xor.
        if matches.opts_present(&["a".to_string(), "m".to_string(), "time".to_string()]) {
            let st = stat(path, !matches.opt_present("no-dereference"));
            let time = matches.opt_strs("time");

            if !(matches.opt_present("a") ||
                 time.contains(&"access".to_string()) ||
                 time.contains(&"atime".to_string()) ||
                 time.contains(&"use".to_string())) {
                atime = st.0;
            }

            if !(matches.opt_present("m") ||
                 time.contains(&"modify".to_string()) ||
                 time.contains(&"mtime".to_string())) {
                mtime = st.1;
            }
        }

        // this follows symlinks and thus does not work correctly for the -h flag
        // need to use lutimes() c function on supported platforms
        match set_file_times(path, atime, mtime) {
            Err(e) => show_warning!("cannot touch '{}': {}", path, e),
            _ => (),
        };
    }

    0
}

fn stat(path: &str, follow: bool) -> (u64, u64) {
    let stat_fn = if follow {
        c_stat
    } else {
        c_lstat
    };
    let mut st: stat_t = unsafe { uninitialized() };
    let result = unsafe { stat_fn(path.as_ptr() as *const c_char, &mut st as *mut stat_t) };

    if result < 0 {
        crash!(1, "failed to get attributes of '{}': {}", path, Error::last_os_error());
    }

    // set_file_times expects milliseconds
    let atime = if st.st_atime_nsec == 0 {
        st.st_atime * 1000
    } else {
        st.st_atime_nsec / 1000
    } as u64;

    // set_file_times expects milliseconds
    let mtime = if st.st_mtime_nsec == 0 {
        st.st_mtime * 1000
    } else {
        st.st_mtime_nsec / 1000
    } as u64;

    (atime, mtime)
}

fn parse_date(str: &str) -> u64 {
    // This isn't actually compatible with GNU touch, but there doesn't seem to
    // be any simple specification for what format this parameter allows and I'm
    // not about to implement GNU parse_datetime.
    // http://git.savannah.gnu.org/gitweb/?p=gnulib.git;a=blob_plain;f=lib/parse-datetime.y
    match time::strptime(str, "%c") {
        Ok(tm) => (tm.to_timespec().sec * 1000) as u64,
        Err(e) => panic!("Unable to parse date\n{}", e)
    }
}

fn parse_timestamp(str: &str) -> u64 {
    let format = match str.chars().count() {
        15 => "%Y%m%d%H%M.%S",
        12 => "%Y%m%d%H%M",
        13 => "%y%m%d%H%M.%S",
        10 => "%y%m%d%H%M",
        11 => "%m%d%H%M.%S",
         8 => "%m%d%H%M",
         _ => panic!("Unknown timestamp format")
    };

    match time::strptime(str, format) {
        Ok(tm) => (tm.to_timespec().sec * 1000) as u64,
        Err(e) => panic!("Unable to parse timestamp\n{}", e)
    }
}

