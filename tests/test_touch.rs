extern crate uu_touch;
use self::uu_touch::filetime::{self, FileTime};

extern crate time;

use common::util::*;

fn get_file_times(at: &AtPath, path: &str) -> (FileTime, FileTime) {
    let m = at.metadata(path);
    (FileTime::from_last_access_time(&m),
     FileTime::from_last_modification_time(&m))
}

fn get_symlink_times(at: &AtPath, path: &str) -> (FileTime, FileTime) {
    let m = at.symlink_metadata(path);
    (FileTime::from_last_access_time(&m),
     FileTime::from_last_modification_time(&m))
}

fn set_file_times(at: &AtPath, path: &str, atime: FileTime, mtime: FileTime) {
    filetime::set_file_times(&at.plus_as_string(path), atime, mtime).unwrap()
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
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_touch_default_file";

    ucmd.arg(file).succeeds().no_stderr();


    assert!(at.file_exists(file));
}

#[test]
fn test_touch_no_create_file_absent() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_touch_no_create_file_absent";

    ucmd.arg("-c").arg(file).succeeds().no_stderr();

    assert!(!at.file_exists(file));
}

#[test]
fn test_touch_no_create_file_exists() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_touch_no_create_file_exists";

    at.touch(file);
    assert!(at.file_exists(file));

    ucmd.arg("-c").arg(file).succeeds().no_stderr();

    assert!(at.file_exists(file));
}

#[test]
fn test_touch_set_mdhm_time() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_touch_set_mdhm_time";

    ucmd.args(&["-t", "01011234", file]).succeeds().no_stderr();

    assert!(at.file_exists(file));

    let start_of_year = str_to_filetime("%Y%m%d%H%M", &format!("{}01010000", 1900+time::now().tm_year));
    let (atime, mtime) = get_file_times(&at, file);
    assert_eq!(atime, mtime);
    assert_eq!(atime.seconds_relative_to_1970() - start_of_year.seconds_relative_to_1970(),
               45240);
    assert_eq!(mtime.seconds_relative_to_1970() - start_of_year.seconds_relative_to_1970(),
               45240);
}

#[test]
fn test_touch_set_mdhms_time() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_touch_set_mdhms_time";

    ucmd.args(&["-t", "01011234.56", file]).succeeds().no_stderr();

    assert!(at.file_exists(file));

    let start_of_year = str_to_filetime("%Y%m%d%H%M.%S", &format!("{}01010000.00", 1900+time::now().tm_year));
    let (atime, mtime) = get_file_times(&at, file);
    assert_eq!(atime, mtime);
    assert_eq!(atime.seconds_relative_to_1970() - start_of_year.seconds_relative_to_1970(),
               45296);
    assert_eq!(mtime.seconds_relative_to_1970() - start_of_year.seconds_relative_to_1970(),
               45296);
}

#[test]
fn test_touch_set_ymdhm_time() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_touch_set_ymdhm_time";

    ucmd.args(&["-t", "1501011234", file]).succeeds().no_stderr();

    assert!(at.file_exists(file));

    let start_of_year = str_to_filetime("%y%m%d%H%M", "1501010000");
    let (atime, mtime) = get_file_times(&at, file);
    assert_eq!(atime, mtime);
    assert_eq!(atime.seconds_relative_to_1970() - start_of_year.seconds_relative_to_1970(),
               45240);
    assert_eq!(mtime.seconds_relative_to_1970() - start_of_year.seconds_relative_to_1970(),
               45240);
}

#[test]
fn test_touch_set_ymdhms_time() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_touch_set_ymdhms_time";

    ucmd.args(&["-t", "1501011234.56", file]).succeeds().no_stderr();

    assert!(at.file_exists(file));

    let start_of_year = str_to_filetime("%y%m%d%H%M.%S", "1501010000.00");
    let (atime, mtime) = get_file_times(&at, file);
    assert_eq!(atime, mtime);
    assert_eq!(atime.seconds_relative_to_1970() - start_of_year.seconds_relative_to_1970(),
               45296);
    assert_eq!(mtime.seconds_relative_to_1970() - start_of_year.seconds_relative_to_1970(),
               45296);
}

#[test]
fn test_touch_set_cymdhm_time() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_touch_set_cymdhm_time";

    ucmd.args(&["-t", "201501011234", file]).succeeds().no_stderr();

    assert!(at.file_exists(file));

    let start_of_year = str_to_filetime("%Y%m%d%H%M", "201501010000");
    let (atime, mtime) = get_file_times(&at, file);
    assert_eq!(atime, mtime);
    assert_eq!(atime.seconds_relative_to_1970() - start_of_year.seconds_relative_to_1970(),
               45240);
    assert_eq!(mtime.seconds_relative_to_1970() - start_of_year.seconds_relative_to_1970(),
               45240);
}

#[test]
fn test_touch_set_cymdhms_time() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_touch_set_cymdhms_time";

    ucmd.args(&["-t", "201501011234.56", file]).succeeds().no_stderr();

    assert!(at.file_exists(file));

    let start_of_year = str_to_filetime("%Y%m%d%H%M.%S", "201501010000.00");
    let (atime, mtime) = get_file_times(&at, file);
    assert_eq!(atime, mtime);
    assert_eq!(atime.seconds_relative_to_1970() - start_of_year.seconds_relative_to_1970(),
               45296);
    assert_eq!(mtime.seconds_relative_to_1970() - start_of_year.seconds_relative_to_1970(),
               45296);
}

#[test]
fn test_touch_set_only_atime() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_touch_set_only_atime";

    ucmd.args(&["-t", "201501011234", "-a", file]).succeeds().no_stderr();

    assert!(at.file_exists(file));

    let start_of_year = str_to_filetime("%Y%m%d%H%M", "201501010000");
    let (atime, mtime) = get_file_times(&at, file);
    assert!(atime != mtime);
    assert_eq!(atime.seconds_relative_to_1970() - start_of_year.seconds_relative_to_1970(),
               45240);
}

#[test]
fn test_touch_set_only_mtime() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_touch_set_only_mtime";

    ucmd.args(&["-t", "201501011234", "-m", file]).succeeds().no_stderr();

    assert!(at.file_exists(file));

    let start_of_year = str_to_filetime("%Y%m%d%H%M", "201501010000");
    let (atime, mtime) = get_file_times(&at, file);
    assert!(atime != mtime);
    assert_eq!(mtime.seconds_relative_to_1970() - start_of_year.seconds_relative_to_1970(),
               45240);
}

#[test]
fn test_touch_set_both() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_touch_set_both";

    ucmd.args(&["-t", "201501011234", "-a", "-m", file]).succeeds().no_stderr();

    assert!(at.file_exists(file));

    let start_of_year = str_to_filetime("%Y%m%d%H%M", "201501010000");
    let (atime, mtime) = get_file_times(&at, file);
    assert_eq!(atime, mtime);
    assert_eq!(atime.seconds_relative_to_1970() - start_of_year.seconds_relative_to_1970(),
               45240);
    assert_eq!(mtime.seconds_relative_to_1970() - start_of_year.seconds_relative_to_1970(),
               45240);
}

#[test]
fn test_touch_no_dereference() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file_a = "test_touch_no_dereference_a";
    let file_b = "test_touch_no_dereference_b";
    let start_of_year = str_to_filetime("%Y%m%d%H%M", "201501010000");
    let end_of_year = str_to_filetime("%Y%m%d%H%M", "201512312359");

    at.touch(file_a);
    set_file_times(&at, file_a, start_of_year, start_of_year);
    at.symlink(file_a, file_b);
    assert!(at.file_exists(file_a));
    assert!(at.is_symlink(file_b));

    ucmd.args(&["-t", "201512312359", "-h", file_b]).succeeds().no_stderr();

    let (atime, mtime) = get_symlink_times(&at, file_b);
    assert_eq!(atime, mtime);
    assert_eq!(atime, end_of_year);
    assert_eq!(mtime, end_of_year);

    let (atime, mtime) = get_file_times(&at, file_a);
    assert_eq!(atime, mtime);
    assert_eq!(atime, start_of_year);
    assert_eq!(mtime, start_of_year);
}

#[test]
fn test_touch_reference() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file_a = "test_touch_reference_a";
    let file_b = "test_touch_reference_b";
    let start_of_year = str_to_filetime("%Y%m%d%H%M", "201501010000");

    at.touch(file_a);
    set_file_times(&at, file_a, start_of_year, start_of_year);
    assert!(at.file_exists(file_a));

    ucmd.args(&["-r", file_a, file_b]).succeeds().no_stderr();

    assert!(at.file_exists(file_b));

    let (atime, mtime) = get_file_times(&at, file_b);
    assert_eq!(atime, mtime);
    assert_eq!(atime, start_of_year);
    assert_eq!(mtime, start_of_year);
}

#[test]
fn test_touch_set_date() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_touch_set_date";

    ucmd.args(&["-d", "Thu Jan 01 12:34:00 2015", file]).succeeds().no_stderr();

    assert!(at.file_exists(file));

    let start_of_year = str_to_filetime("%Y%m%d%H%M", "201501011234");
    let (atime, mtime) = get_file_times(&at, file);
    assert_eq!(atime, mtime);
    assert_eq!(atime, start_of_year);
    assert_eq!(mtime, start_of_year);
}
