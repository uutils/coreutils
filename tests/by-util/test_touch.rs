// spell-checker:ignore (formats) cymdhm cymdhms mdhm mdhms ymdhm ymdhms

extern crate touch;
use self::touch::filetime::{self, FileTime};

extern crate time;

use crate::common::util::*;
use std::fs::remove_file;
use std::path::PathBuf;

fn get_file_times(at: &AtPath, path: &str) -> (FileTime, FileTime) {
    let m = at.metadata(path);
    (
        FileTime::from_last_access_time(&m),
        FileTime::from_last_modification_time(&m),
    )
}

#[cfg(not(target_os = "freebsd"))]
fn get_symlink_times(at: &AtPath, path: &str) -> (FileTime, FileTime) {
    let m = at.symlink_metadata(path);
    (
        FileTime::from_last_access_time(&m),
        FileTime::from_last_modification_time(&m),
    )
}

fn set_file_times(at: &AtPath, path: &str, atime: FileTime, mtime: FileTime) {
    filetime::set_file_times(&at.plus_as_string(path), atime, mtime).unwrap();
}

// Adjusts for local timezone
fn str_to_filetime(format: &str, s: &str) -> FileTime {
    let mut tm = time::strptime(s, format).unwrap();
    tm.tm_utcoff = time::now().tm_utcoff;
    tm.tm_isdst = -1; // Unknown flag DST
    let ts = tm.to_timespec();
    FileTime::from_unix_time(ts.sec as i64, ts.nsec as u32)
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

    let start_of_year = str_to_filetime(
        "%Y%m%d%H%M",
        &format!("{}01010000", 1900 + time::now().tm_year),
    );
    let (atime, mtime) = get_file_times(&at, file);
    assert_eq!(atime, mtime);
    assert_eq!(atime.unix_seconds() - start_of_year.unix_seconds(), 45240);
    assert_eq!(mtime.unix_seconds() - start_of_year.unix_seconds(), 45240);
}

#[test]
fn test_touch_set_mdhms_time() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_touch_set_mdhms_time";

    ucmd.args(&["-t", "01011234.56", file])
        .succeeds()
        .no_stderr();

    assert!(at.file_exists(file));

    let start_of_year = str_to_filetime(
        "%Y%m%d%H%M.%S",
        &format!("{}01010000.00", 1900 + time::now().tm_year),
    );
    let (atime, mtime) = get_file_times(&at, file);
    assert_eq!(atime, mtime);
    assert_eq!(atime.unix_seconds() - start_of_year.unix_seconds(), 45296);
    assert_eq!(mtime.unix_seconds() - start_of_year.unix_seconds(), 45296);
}

#[test]
fn test_touch_set_ymdhm_time() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_touch_set_ymdhm_time";

    ucmd.args(&["-t", "1501011234", file])
        .succeeds()
        .no_stderr();

    assert!(at.file_exists(file));

    let start_of_year = str_to_filetime("%y%m%d%H%M", "1501010000");
    let (atime, mtime) = get_file_times(&at, file);
    assert_eq!(atime, mtime);
    assert_eq!(atime.unix_seconds() - start_of_year.unix_seconds(), 45240);
    assert_eq!(mtime.unix_seconds() - start_of_year.unix_seconds(), 45240);
}

#[test]
fn test_touch_set_ymdhms_time() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_touch_set_ymdhms_time";

    ucmd.args(&["-t", "1501011234.56", file])
        .succeeds()
        .no_stderr();

    assert!(at.file_exists(file));

    let start_of_year = str_to_filetime("%y%m%d%H%M.%S", "1501010000.00");
    let (atime, mtime) = get_file_times(&at, file);
    assert_eq!(atime, mtime);
    assert_eq!(atime.unix_seconds() - start_of_year.unix_seconds(), 45296);
    assert_eq!(mtime.unix_seconds() - start_of_year.unix_seconds(), 45296);
}

#[test]
fn test_touch_set_cymdhm_time() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_touch_set_cymdhm_time";

    ucmd.args(&["-t", "201501011234", file])
        .succeeds()
        .no_stderr();

    assert!(at.file_exists(file));

    let start_of_year = str_to_filetime("%Y%m%d%H%M", "201501010000");
    let (atime, mtime) = get_file_times(&at, file);
    assert_eq!(atime, mtime);
    assert_eq!(atime.unix_seconds() - start_of_year.unix_seconds(), 45240);
    assert_eq!(mtime.unix_seconds() - start_of_year.unix_seconds(), 45240);
}

#[test]
fn test_touch_set_cymdhms_time() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_touch_set_cymdhms_time";

    ucmd.args(&["-t", "201501011234.56", file])
        .succeeds()
        .no_stderr();

    assert!(at.file_exists(file));

    let start_of_year = str_to_filetime("%Y%m%d%H%M.%S", "201501010000.00");
    let (atime, mtime) = get_file_times(&at, file);
    assert_eq!(atime, mtime);
    assert_eq!(atime.unix_seconds() - start_of_year.unix_seconds(), 45296);
    assert_eq!(mtime.unix_seconds() - start_of_year.unix_seconds(), 45296);
}

#[test]
fn test_touch_set_only_atime() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_touch_set_only_atime";

    ucmd.args(&["-t", "201501011234", "-a", file])
        .succeeds()
        .no_stderr();

    assert!(at.file_exists(file));

    let start_of_year = str_to_filetime("%Y%m%d%H%M", "201501010000");
    let (atime, mtime) = get_file_times(&at, file);
    assert!(atime != mtime);
    assert_eq!(atime.unix_seconds() - start_of_year.unix_seconds(), 45240);
}

#[test]
fn test_touch_set_only_mtime_failed() {
    let (_at, mut ucmd) = at_and_ucmd!();
    let file = "test_touch_set_only_mtime";

    ucmd.args(&["-t", "2015010112342", "-m", file]).fails();
}

#[test]
fn test_touch_set_both_time_and_reference() {
    let (at, mut ucmd) = at_and_ucmd!();
    let ref_file = "test_touch_reference";
    let file = "test_touch_set_both_time_and_reference";

    let start_of_year = str_to_filetime("%Y%m%d%H%M", "201501010000");

    at.touch(ref_file);
    set_file_times(&at, ref_file, start_of_year, start_of_year);
    assert!(at.file_exists(ref_file));

    ucmd.args(&["-t", "2015010112342", "-r", ref_file, file])
        .fails();
}

#[test]
fn test_touch_set_both_date_and_reference() {
    let (at, mut ucmd) = at_and_ucmd!();
    let ref_file = "test_touch_reference";
    let file = "test_touch_set_both_date_and_reference";

    let start_of_year = str_to_filetime("%Y%m%d%H%M", "201501010000");

    at.touch(ref_file);
    set_file_times(&at, ref_file, start_of_year, start_of_year);
    assert!(at.file_exists(ref_file));

    ucmd.args(&["-d", "Thu Jan 01 12:34:00 2015", "-r", ref_file, file])
        .fails();
}

#[test]
fn test_touch_set_both_time_and_date() {
    let (_at, mut ucmd) = at_and_ucmd!();
    let file = "test_touch_set_both_time_and_date";

    ucmd.args(&[
        "-t",
        "2015010112342",
        "-d",
        "Thu Jan 01 12:34:00 2015",
        file,
    ])
    .fails();
}

#[test]
fn test_touch_set_only_mtime() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_touch_set_only_mtime";

    ucmd.args(&["-t", "201501011234", "-m", file])
        .succeeds()
        .no_stderr();

    assert!(at.file_exists(file));

    let start_of_year = str_to_filetime("%Y%m%d%H%M", "201501010000");
    let (atime, mtime) = get_file_times(&at, file);
    assert!(atime != mtime);
    assert_eq!(mtime.unix_seconds() - start_of_year.unix_seconds(), 45240);
}

#[test]
fn test_touch_set_both() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_touch_set_both";

    ucmd.args(&["-t", "201501011234", "-a", "-m", file])
        .succeeds()
        .no_stderr();

    assert!(at.file_exists(file));

    let start_of_year = str_to_filetime("%Y%m%d%H%M", "201501010000");
    let (atime, mtime) = get_file_times(&at, file);
    assert_eq!(atime, mtime);
    assert_eq!(atime.unix_seconds() - start_of_year.unix_seconds(), 45240);
    assert_eq!(mtime.unix_seconds() - start_of_year.unix_seconds(), 45240);
}

#[test]
// FixME: Fails on freebsd because of a different nanos
#[cfg(not(target_os = "freebsd"))]
fn test_touch_no_dereference() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file_a = "test_touch_no_dereference_a";
    let file_b = "test_touch_no_dereference_b";
    let start_of_year = str_to_filetime("%Y%m%d%H%M", "201501010000");
    let end_of_year = str_to_filetime("%Y%m%d%H%M", "201512312359");

    at.touch(file_a);
    set_file_times(&at, file_a, start_of_year, start_of_year);
    at.symlink_file(file_a, file_b);
    assert!(at.file_exists(file_a));
    assert!(at.is_symlink(file_b));

    ucmd.args(&["-t", "201512312359", "-h", file_b])
        .succeeds()
        .no_stderr();

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
    let scenario = TestScenario::new("touch");
    let (at, mut _ucmd) = (scenario.fixtures.clone(), scenario.ucmd());
    let file_a = "test_touch_reference_a";
    let file_b = "test_touch_reference_b";
    let start_of_year = str_to_filetime("%Y%m%d%H%M", "201501010000");

    at.touch(file_a);
    set_file_times(&at, file_a, start_of_year, start_of_year);
    assert!(at.file_exists(file_a));
    for opt in ["-r", "--ref", "--reference"] {
        scenario
            .ccmd("touch")
            .args(&[opt, file_a, file_b])
            .succeeds()
            .no_stderr();

        assert!(at.file_exists(file_b));

        let (atime, mtime) = get_file_times(&at, file_b);
        assert_eq!(atime, mtime);
        assert_eq!(atime, start_of_year);
        assert_eq!(mtime, start_of_year);
        let _ = remove_file(file_b);
    }
}

#[test]
fn test_touch_set_date() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_touch_set_date";

    ucmd.args(&["-d", "Thu Jan 01 12:34:00 2015", file])
        .succeeds()
        .no_stderr();

    assert!(at.file_exists(file));

    let start_of_year = str_to_filetime("%Y%m%d%H%M", "201501011234");
    let (atime, mtime) = get_file_times(&at, file);
    assert_eq!(atime, mtime);
    assert_eq!(atime, start_of_year);
    assert_eq!(mtime, start_of_year);
}

#[test]
fn test_touch_set_date2() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_touch_set_date";

    ucmd.args(&["-d", "2000-01-23", file])
        .succeeds()
        .no_stderr();

    assert!(at.file_exists(file));

    let start_of_year = str_to_filetime("%Y%m%d%H%M", "200001230000");
    let (atime, mtime) = get_file_times(&at, file);
    assert_eq!(atime, mtime);
    assert_eq!(atime, start_of_year);
    assert_eq!(mtime, start_of_year);
}

#[test]
fn test_touch_set_date3() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_touch_set_date";

    ucmd.args(&["-d", "@1623786360", file])
        .succeeds()
        .no_stderr();

    assert!(at.file_exists(file));

    let expected = FileTime::from_unix_time(1623786360, 0);
    let (atime, mtime) = get_file_times(&at, file);
    assert_eq!(atime, mtime);
    assert_eq!(atime, expected);
    assert_eq!(mtime, expected);
}

#[test]
fn test_touch_set_date_wrong_format() {
    let (_at, mut ucmd) = at_and_ucmd!();
    let file = "test_touch_set_date_wrong_format";

    ucmd.args(&["-d", "2005-43-21", file])
        .fails()
        .stderr_contains("Unable to parse date: 2005-43-21");
}

#[test]
fn test_touch_mtime_dst_succeeds() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_touch_set_mtime_dst_succeeds";

    ucmd.args(&["-m", "-t", "202103140300", file])
        .succeeds()
        .no_stderr();

    assert!(at.file_exists(file));

    let target_time = str_to_filetime("%Y%m%d%H%M", "202103140300");
    let (_, mtime) = get_file_times(&at, file);
    assert_eq!(target_time, mtime);
}

// is_dst_switch_hour returns true if timespec ts is just before the switch
// to Daylight Saving Time.
// For example, in EST (UTC-5), Timespec { sec: 1583647200, nsec: 0 }
// for March 8 2020 01:00:00 AM
// is just before the switch because on that day clock jumps by 1 hour,
// so 1 minute after 01:59:00 is 03:00:00.
fn is_dst_switch_hour(ts: time::Timespec) -> bool {
    let ts_after = ts + time::Duration::hours(1);
    let tm = time::at(ts);
    let tm_after = time::at(ts_after);
    tm_after.tm_hour == tm.tm_hour + 2
}

// get_dst_switch_hour returns date string for which touch -m -t fails.
// For example, in EST (UTC-5), that will be "202003080200" so
// touch -m -t 202003080200 file
// fails (that date/time does not exist).
// In other locales it will be a different date/time, and in some locales
// it doesn't exist at all, in which case this function will return None.
fn get_dst_switch_hour() -> Option<String> {
    let now = time::now();
    // Start from January 1, 2020, 00:00.
    let mut tm = time::strptime("20200101-0000", "%Y%m%d-%H%M").unwrap();
    tm.tm_isdst = -1;
    tm.tm_utcoff = now.tm_utcoff;
    let mut ts = tm.to_timespec();
    // Loop through all hours in year 2020 until we find the hour just
    // before the switch to DST.
    for _i in 0..(366 * 24) {
        if is_dst_switch_hour(ts) {
            let mut tm = time::at(ts);
            tm.tm_hour += 1;
            let s = time::strftime("%Y%m%d%H%M", &tm).unwrap();
            return Some(s);
        }
        ts = ts + time::Duration::hours(1);
    }
    None
}

#[test]
fn test_touch_mtime_dst_fails() {
    let (_at, mut ucmd) = at_and_ucmd!();
    let file = "test_touch_set_mtime_dst_fails";

    if let Some(s) = get_dst_switch_hour() {
        ucmd.args(&["-m", "-t", &s, file]).fails();
    }
}

#[test]
#[cfg(unix)]
fn test_touch_system_fails() {
    let (_at, mut ucmd) = at_and_ucmd!();
    let file = "/";
    ucmd.args(&[file])
        .fails()
        .stderr_contains("setting times of '/'");
}

#[test]
fn test_touch_trailing_slash() {
    let (_at, mut ucmd) = at_and_ucmd!();
    let file = "no-file/";
    ucmd.args(&[file]).fails();
}

#[test]
fn test_touch_no_such_file_error_msg() {
    let dirname = "nonexistent";
    let filename = "file";
    let path = PathBuf::from(dirname).join(filename);
    let path_str = path.to_str().unwrap();

    new_ucmd!().arg(&path).fails().stderr_only(format!(
        "touch: cannot touch '{}': No such file or directory",
        path_str
    ));
}

#[test]
fn test_touch_changes_time_of_file_in_stdout() {
    // command like: `touch - 1< ./c`
    // should change the timestamp of c

    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_touch_changes_time_of_file_in_stdout";

    at.touch(file);
    assert!(at.file_exists(file));
    let (_, mtime) = get_file_times(&at, file);

    ucmd.args(&["-"])
        .set_stdout(at.make_file(file))
        .succeeds()
        .no_stderr();

    let (_, mtime_after) = get_file_times(&at, file);
    assert!(mtime_after != mtime);
}

#[test]
#[cfg(unix)]
fn test_touch_permission_denied_error_msg() {
    let (at, mut ucmd) = at_and_ucmd!();

    let dirname = "dir_with_read_only_access";
    let filename = "file";
    let path = PathBuf::from(dirname).join(filename);
    let path_str = path.to_str().unwrap();

    // create dest without write permissions
    at.mkdir(dirname);
    at.set_readonly(dirname);

    let full_path = at.plus_as_string(path_str);
    ucmd.arg(&full_path).fails().stderr_only(format!(
        "touch: cannot touch '{}': Permission denied",
        &full_path
    ));
}

#[test]
fn test_touch_no_args() {
    let mut ucmd = new_ucmd!();
    ucmd.fails().stderr_only(
        r##"touch: missing file operand
Try 'touch --help' for more information."##,
    );
}

#[test]
fn test_no_dereference_no_file() {
    new_ucmd!()
        .args(&["-h", "not-a-file"])
        .fails()
        .stderr_contains("setting times of 'not-a-file': No such file or directory");
    new_ucmd!()
        .args(&["-h", "not-a-file-1", "not-a-file-2"])
        .fails()
        .stderr_contains("setting times of 'not-a-file-1': No such file or directory")
        .stderr_contains("setting times of 'not-a-file-2': No such file or directory");
}
