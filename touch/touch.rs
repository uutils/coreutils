#![crate_id(name="touch", vers="1.0.0", author="Nick Platt")]
#![feature(macro_rules)]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Nick Platt <platt.nicholas@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE file
 * that was distributed with this source code.
 */

extern crate getopts;
extern crate time;

use std::io::File;
use std::os;

#[path = "../common/util.rs"]
mod util;

static NAME: &'static str = "touch";
static VERSION: &'static str = "1.0.0";

#[allow(dead_code)]
fn main() { os::set_exit_status(uumain(os::args())); }

pub fn uumain(args: Vec<String>) -> int {
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

    let matches = match getopts::getopts(args.tail(), opts) {
        Ok(m)  => m,
        Err(e) => fail!("Invalid options\n{}", e.to_err_msg())
    };

    if matches.opt_present("version") {
        println!("{:s} {:s}", NAME, VERSION);
        return 0;
    }

    if matches.opt_present("help") || matches.free.is_empty() {
        println!("{:s} {:s}", NAME, VERSION);
        println!("");
        println!("Usage: {:s} [OPTION]... FILE...", NAME);
        println!("");
        println!("{:s}", getopts::usage("Update the access and modification times of \
                                         each FILE to the current time.", opts));
        return 0;
    }

    if matches.opt_present("date") && matches.opts_present(["reference".to_string(), "t".to_string()]) ||
       matches.opt_present("reference") && matches.opts_present(["date".to_string(), "t".to_string()]) ||
       matches.opt_present("t") && matches.opts_present(["date".to_string(), "reference".to_string()]) {
        fail!("Invalid options: cannot specify reference time from more than one source");
    }

    let (mut atime, mut mtime) =
        if matches.opt_present("reference") {
            let path = Path::new(matches.opt_str("reference").unwrap().to_string());
            let stat = stat(&path, !matches.opt_present("no-dereference"));
            (stat.accessed, stat.modified)
        } else if matches.opts_present(["date".to_string(), "t".to_string()]) {
            let timestamp = if matches.opt_present("date") {
                parse_date(matches.opt_str("date").unwrap().as_slice())
            } else {
                parse_timestamp(matches.opt_str("t").unwrap().as_slice())
            };
            (timestamp, timestamp)
        } else {
            // FIXME: Should use Timespec. https://github.com/mozilla/rust/issues/10301
            let now = (time::get_time().sec * 1000) as u64;
            (now, now)
        };

    for filename in matches.free.iter() {
        let path = Path::new(filename.to_string());

        if !path.exists() {
            // no-dereference included here for compatibility
            if matches.opts_present(["no-create".to_string(), "no-dereference".to_string()]) {
                continue;
            }

            match File::create(&path) {
                Ok(fd) => fd,
                Err(e) => fail!("Unable to create file: {}\n{}", filename, e.desc)
            };

            // Minor optimization: if no reference time was specified, we're done.
            if !matches.opts_present(["date".to_string(), "reference".to_string(), "t".to_string()]) {
                continue;
            }
        }

        // If changing "only" atime or mtime, grab the existing value of the other.
        // Note that "-a" and "-m" may be passed together; this is not an xor.
        if matches.opts_present(["a".to_string(), "m".to_string(), "time".to_string()]) {
            let stat = stat(&path, !matches.opt_present("no-dereference"));
            let time = matches.opt_strs("time");

            if !(matches.opt_present("a") ||
                 time.contains(&"access".to_string()) ||
                 time.contains(&"atime".to_string()) ||
                 time.contains(&"use".to_string())) {
                atime = stat.accessed;
            }

            if !(matches.opt_present("m") ||
                 time.contains(&"modify".to_string()) ||
                 time.contains(&"mtime".to_string())) {
                mtime = stat.modified;
            }
        }

        match std::io::fs::change_file_times(&path, atime, mtime) {
            Ok(t) => t,
            Err(e) => fail!("Unable to modify times\n{}", e.desc)
        }
    }

    return 0;
}

fn stat(path: &Path, follow: bool) -> std::io::FileStat {
    if follow {
        match std::io::fs::stat(path) {
            Ok(stat) => stat,
            Err(e)   => fail!("Unable to open file\n{}", e.desc)
        }
    } else {
        match std::io::fs::lstat(path) {
            Ok(stat) => stat,
            Err(e)   => fail!("Unable to open file\n{}", e.desc)
        }
    }
}

fn parse_date(str: &str) -> u64 {
    // This isn't actually compatible with GNU touch, but there doesn't seem to
    // be any simple specification for what format this parameter allows and I'm
    // not about to implement GNU parse_datetime.
    // http://git.savannah.gnu.org/gitweb/?p=gnulib.git;a=blob_plain;f=lib/parse-datetime.y
    match time::strptime(str, "%c") {
        Ok(tm) => (tm.to_timespec().sec * 1000) as u64,
        Err(e) => fail!("Unable to parse date\n{}", e)
    }
}

fn parse_timestamp(str: &str) -> u64 {
    let format = match str.char_len() {
        15 => "%Y%m%d%H%M.%S",
        12 => "%Y%m%d%H%M",
        13 => "%y%m%d%H%M.%S",
        10 => "%y%m%d%H%M",
        11 => "%m%d%H%M.%S",
         8 => "%m%d%H%M",
         _ => fail!("Unknown timestamp format")
    };

    match time::strptime(str, format) {
        Ok(tm) => (tm.to_timespec().sec * 1000) as u64,
        Err(e) => fail!("Unable to parse timestamp\n{}", e)
    }
}

