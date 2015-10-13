extern crate libc;
extern crate time;
extern crate kernel32;
extern crate winapi;
extern crate filetime;

use filetime::FileTime;
use std::process::Command;
use util::*;

static PROGNAME: &'static str = "./touch";

#[path = "common/util.rs"]
#[macro_use]
mod util;

fn get_file_times(path: &str) -> (FileTime, FileTime) {
    let m = metadata(path);
    (FileTime::from_last_access_time(&m), FileTime::from_last_modification_time(&m))
}

fn set_file_times(path: &str, atime: FileTime, mtime: FileTime) {
    filetime::set_file_times(path, atime, mtime).unwrap()
}

// Adjusts for local timezone
fn str_to_filetime(format: &str, s: &str) -> FileTime {
    let mut tm = time::strptime(s, format).unwrap();
    tm.tm_utcoff = time::now().tm_utcoff;
    let ts = tm.to_timespec();
    FileTime::from_seconds_since_1970(ts.sec as u64, ts.nsec as u32)
}

#[test]
fn test_touch_default() {
    let file = "test_touch_default_file";

    let result = run(Command::new(PROGNAME).arg(file));
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(file_exists(file));
}

#[test]
fn test_touch_no_create_file_absent() {
    let file = "test_touch_no_create_file_absent";

    let result = run(Command::new(PROGNAME).arg("-c").arg(file));
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(!file_exists(file));
}

#[test]
fn test_touch_no_create_file_exists() {
    let file = "test_touch_no_create_file_exists";

    touch(file);
    assert!(file_exists(file));

    let result = run(Command::new(PROGNAME).arg("-c").arg(file));
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(file_exists(file));
}

#[test]
fn test_touch_set_mdhm_time() {
    let file = "test_touch_set_mdhm_time";

    let result = run(Command::new(PROGNAME).args(&["-t", "01011234", file]));
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(file_exists(file));

    let start_of_year = str_to_filetime("%Y%m%d%H%M", "201501010000");
    let (atime, mtime) = get_file_times(file);
    assert_eq!(atime, mtime);
    assert_eq!(atime.seconds_relative_to_1970() - start_of_year.seconds_relative_to_1970(), 45240);
    assert_eq!(mtime.seconds_relative_to_1970() - start_of_year.seconds_relative_to_1970(), 45240);
}

#[test]
fn test_touch_set_mdhms_time() {
    let file = "test_touch_set_mdhms_time";

    let result = run(Command::new(PROGNAME).args(&["-t", "01011234.56", file]));
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(file_exists(file));

    let start_of_year = str_to_filetime("%Y%m%d%H%M.%S", "201501010000.00");
    let (atime, mtime) = get_file_times(file);
    assert_eq!(atime, mtime);
    assert_eq!(atime.seconds_relative_to_1970() - start_of_year.seconds_relative_to_1970(), 45296);
    assert_eq!(mtime.seconds_relative_to_1970() - start_of_year.seconds_relative_to_1970(), 45296);
}

#[test]
fn test_touch_set_ymdhm_time() {
    let file = "test_touch_set_ymdhm_time";

    let result = run(Command::new(PROGNAME).args(&["-t", "1501011234", file]));
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(file_exists(file));

    let start_of_year = str_to_filetime("%y%m%d%H%M", "1501010000");
    let (atime, mtime) = get_file_times(file);
    assert_eq!(atime, mtime);
    assert_eq!(atime.seconds_relative_to_1970() - start_of_year.seconds_relative_to_1970(), 45240);
    assert_eq!(mtime.seconds_relative_to_1970() - start_of_year.seconds_relative_to_1970(), 45240);
}

#[test]
fn test_touch_set_ymdhms_time() {
    let file = "test_touch_set_ymdhms_time";

    let result = run(Command::new(PROGNAME).args(&["-t", "1501011234.56", file]));
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(file_exists(file));

    let start_of_year = str_to_filetime("%y%m%d%H%M.%S", "1501010000.00");
    let (atime, mtime) = get_file_times(file);
    assert_eq!(atime, mtime);
    assert_eq!(atime.seconds_relative_to_1970() - start_of_year.seconds_relative_to_1970(), 45296);
    assert_eq!(mtime.seconds_relative_to_1970() - start_of_year.seconds_relative_to_1970(), 45296);
}

#[test]
fn test_touch_set_cymdhm_time() {
    let file = "test_touch_set_cymdhm_time";

    let result = run(Command::new(PROGNAME).args(&["-t", "201501011234", file]));
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(file_exists(file));

    let start_of_year = str_to_filetime("%Y%m%d%H%M", "201501010000");
    let (atime, mtime) = get_file_times(file);
    assert_eq!(atime, mtime);
    assert_eq!(atime.seconds_relative_to_1970() - start_of_year.seconds_relative_to_1970(), 45240);
    assert_eq!(mtime.seconds_relative_to_1970() - start_of_year.seconds_relative_to_1970(), 45240);
}

#[test]
fn test_touch_set_cymdhms_time() {
    let file = "test_touch_set_cymdhms_time";

    let result = run(Command::new(PROGNAME).args(&["-t", "201501011234.56", file]));
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(file_exists(file));

    let start_of_year = str_to_filetime("%Y%m%d%H%M.%S", "201501010000.00");
    let (atime, mtime) = get_file_times(file);
    assert_eq!(atime, mtime);
    assert_eq!(atime.seconds_relative_to_1970() - start_of_year.seconds_relative_to_1970(), 45296);
    assert_eq!(mtime.seconds_relative_to_1970() - start_of_year.seconds_relative_to_1970(), 45296);
}

#[test]
fn test_touch_set_only_atime() {
    let file = "test_touch_set_only_atime";

    let result = run(Command::new(PROGNAME).args(&["-t", "201501011234", "-a", file]));
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(file_exists(file));

    let start_of_year = str_to_filetime("%Y%m%d%H%M", "201501010000");
    let (atime, mtime) = get_file_times(file);
    assert!(atime != mtime);
    assert_eq!(atime.seconds_relative_to_1970() - start_of_year.seconds_relative_to_1970(), 45240);
}

#[test]
fn test_touch_set_only_mtime() {
    let file = "test_touch_set_only_mtime";

    let result = run(Command::new(PROGNAME).args(&["-t", "201501011234", "-m", file]));
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(file_exists(file));

    let start_of_year = str_to_filetime("%Y%m%d%H%M", "201501010000");
    let (atime, mtime) = get_file_times(file);
    assert!(atime != mtime);
    assert_eq!(mtime.seconds_relative_to_1970() - start_of_year.seconds_relative_to_1970(), 45240);
}

#[test]
fn test_touch_set_both() {
    let file = "test_touch_set_both";

    let result = run(Command::new(PROGNAME).args(&["-t", "201501011234", "-a", "-m", file]));
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(file_exists(file));

    let start_of_year = str_to_filetime("%Y%m%d%H%M", "201501010000");
    let (atime, mtime) = get_file_times(file);
    assert_eq!(atime, mtime);
    assert_eq!(atime.seconds_relative_to_1970() - start_of_year.seconds_relative_to_1970(), 45240);
    assert_eq!(mtime.seconds_relative_to_1970() - start_of_year.seconds_relative_to_1970(), 45240);
}

#[test]
fn test_touch_reference() {
    let file_a = "test_touch_reference_a";
    let file_b = "test_touch_reference_b";
    let start_of_year = str_to_filetime("%Y%m%d%H%M", "201501010000");

    touch(file_a);
    set_file_times(file_a, start_of_year, start_of_year);
    assert!(file_exists(file_a));

    let result = run(Command::new(PROGNAME).args(&["-r", file_a, file_b]));
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(file_exists(file_b));

    let (atime, mtime) = get_file_times(file_b);
    assert_eq!(atime, mtime);
    assert_eq!(atime, start_of_year);
    assert_eq!(mtime, start_of_year);
}

#[test]
fn test_touch_set_date() {
    let file = "test_touch_set_date";

    let result = run(Command::new(PROGNAME).args(&["-d", "Thu Jan 01 12:34:00 2015", file]));
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(file_exists(file));

    let start_of_year = str_to_filetime("%Y%m%d%H%M", "201501011234");
    let (atime, mtime) = get_file_times(file);
    assert_eq!(atime, mtime);
    assert_eq!(atime, start_of_year);
    assert_eq!(mtime, start_of_year);
}
