// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore (formats) cymdhm cymdhms mdhm mdhms ymdhm ymdhms datetime mktime

#[cfg(not(target_os = "freebsd"))]
use filetime::set_symlink_file_times;
use filetime::FileTime;
use std::fs::remove_file;
use std::path::PathBuf;
use uutests::at_and_ucmd;
use uutests::new_ucmd;
use uutests::util::{AtPath, TestScenario};
use uutests::util_name;

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
    filetime::set_file_times(at.plus_as_string(path), atime, mtime).unwrap();
}

fn str_to_filetime(format: &str, s: &str) -> FileTime {
    let tm = chrono::NaiveDateTime::parse_from_str(s, format).unwrap();
    FileTime::from_unix_time(
        tm.and_utc().timestamp(),
        tm.and_utc().timestamp_subsec_nanos(),
    )
}

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails().code_is(1);
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
        &format!("{}01010000", time::OffsetDateTime::now_utc().year()),
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
        &format!("{}01010000.00", time::OffsetDateTime::now_utc().year()),
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

    let start_of_year = str_to_filetime("%Y%m%d%H%M", "201501010000");
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

    let start_of_year = str_to_filetime("%Y%m%d%H%M.%S", "201501010000.00");
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
    let atime_args = [
        "-a",
        "--time=access",
        "--time=atime",
        "--time=atim", // spell-checker:disable-line
        "--time=a",
        "--time=use",
    ];
    let file = "test_touch_set_only_atime";

    for atime_arg in atime_args {
        let (at, mut ucmd) = at_and_ucmd!();

        ucmd.args(&["-t", "201501011234", atime_arg, file])
            .succeeds()
            .no_stderr();

        assert!(at.file_exists(file));

        let start_of_year = str_to_filetime("%Y%m%d%H%M", "201501010000");
        let (atime, mtime) = get_file_times(&at, file);
        assert!(atime != mtime);
        assert_eq!(atime.unix_seconds() - start_of_year.unix_seconds(), 45240);
    }
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

    let start_of_year = str_to_filetime("%Y%m%d%H%M", "201501011234");

    at.touch(ref_file);
    set_file_times(&at, ref_file, start_of_year, start_of_year);
    assert!(at.file_exists(ref_file));

    ucmd.args(&["-d", "Thu Jan 01 12:34:00 2015", "-r", ref_file, file])
        .succeeds()
        .no_stderr();
    let (atime, mtime) = get_file_times(&at, file);
    assert_eq!(atime, start_of_year);
    assert_eq!(mtime, start_of_year);
}

#[test]
fn test_touch_set_both_offset_date_and_reference() {
    let (at, mut ucmd) = at_and_ucmd!();
    let ref_file = "test_touch_reference";
    let file = "test_touch_set_both_date_and_reference";

    let start_of_year = str_to_filetime("%Y%m%d%H%M", "201501011234");
    let five_days_later = str_to_filetime("%Y%m%d%H%M", "201501061234");

    at.touch(ref_file);
    set_file_times(&at, ref_file, start_of_year, start_of_year);
    assert!(at.file_exists(ref_file));

    ucmd.args(&["-d", "+5 days", "-r", ref_file, file])
        .succeeds()
        .no_stderr();
    let (atime, mtime) = get_file_times(&at, file);
    assert_eq!(atime, five_days_later);
    assert_eq!(mtime, five_days_later);
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
    let mtime_args = ["-m", "--time=modify", "--time=mtime", "--time=m"];
    let file = "test_touch_set_only_mtime";

    for mtime_arg in mtime_args {
        let (at, mut ucmd) = at_and_ucmd!();

        ucmd.args(&["-t", "201501011234", mtime_arg, file])
            .succeeds()
            .no_stderr();

        assert!(at.file_exists(file));

        let start_of_year = str_to_filetime("%Y%m%d%H%M", "201501010000");
        let (atime, mtime) = get_file_times(&at, file);
        assert!(atime != mtime);
        assert_eq!(mtime.unix_seconds() - start_of_year.unix_seconds(), 45240);
    }
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

    let expected = FileTime::from_unix_time(1_623_786_360, 0);
    let (atime, mtime) = get_file_times(&at, file);
    assert_eq!(atime, mtime);
    assert_eq!(atime, expected);
    assert_eq!(mtime, expected);
}

#[test]
fn test_touch_set_date4() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_touch_set_date";

    ucmd.args(&["-d", "1970-01-01 18:43:33", file])
        .succeeds()
        .no_stderr();

    assert!(at.file_exists(file));

    let expected = FileTime::from_unix_time(67413, 0);
    let (atime, mtime) = get_file_times(&at, file);
    assert_eq!(atime, mtime);
    assert_eq!(atime, expected);
    assert_eq!(mtime, expected);
}

#[test]
fn test_touch_set_date5() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_touch_set_date";

    ucmd.args(&["-d", "1970-01-01 18:43:33.023456789", file])
        .succeeds()
        .no_stderr();

    assert!(at.file_exists(file));

    // Slightly different result on Windows for nano seconds
    // TODO: investigate
    #[cfg(windows)]
    let expected = FileTime::from_unix_time(67413, 23_456_700);
    #[cfg(not(windows))]
    let expected = FileTime::from_unix_time(67413, 23_456_789);

    let (atime, mtime) = get_file_times(&at, file);
    assert_eq!(atime, mtime);
    assert_eq!(atime, expected);
    assert_eq!(mtime, expected);
}

#[test]
fn test_touch_set_date6() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_touch_set_date";

    ucmd.args(&["-d", "2000-01-01 00:00", file])
        .succeeds()
        .no_stderr();

    assert!(at.file_exists(file));

    let expected = FileTime::from_unix_time(946_684_800, 0);

    let (atime, mtime) = get_file_times(&at, file);
    assert_eq!(atime, mtime);
    assert_eq!(atime, expected);
    assert_eq!(mtime, expected);
}

#[test]
fn test_touch_set_date7() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_touch_set_date";

    ucmd.args(&["-d", "2004-01-16 12:00 +0000", file])
        .succeeds()
        .no_stderr();

    assert!(at.file_exists(file));

    let expected = FileTime::from_unix_time(1_074_254_400, 0);

    let (atime, mtime) = get_file_times(&at, file);
    assert_eq!(atime, mtime);
    assert_eq!(atime, expected);
    assert_eq!(mtime, expected);
}

/// Test for setting the date by a relative time unit.
#[test]
fn test_touch_set_date_relative_smoke() {
    // From the GNU documentation:
    //
    // > The unit of time displacement may be selected by the string
    // > ‘year’ or ‘month’ for moving by whole years or months.  These
    // > are fuzzy units, as years and months are not all of equal
    // > duration.  More precise units are ‘fortnight’ which is worth 14
    // > days, ‘week’ worth 7 days, ‘day’ worth 24 hours, ‘hour’ worth
    // > 60 minutes, ‘minute’ or ‘min’ worth 60 seconds, and ‘second’ or
    // > ‘sec’ worth one second.  An ‘s’ suffix on these units is
    // > accepted and ignored.
    //
    let times = [
        // "-1 year", "+1 year", "-1 years", "+1 years",
        // "-1 month", "+1 month", "-1 months", "+1 months",
        "-1 fortnight",
        "+1 fortnight",
        "-1 fortnights",
        "+1 fortnights",
        "fortnight",
        "fortnights",
        "-1 week",
        "+1 week",
        "-1 weeks",
        "+1 weeks",
        "week",
        "weeks",
        "-1 day",
        "+1 day",
        "-1 days",
        "+1 days",
        "day",
        "days",
        "-1 hour",
        "+1 hour",
        "-1 hours",
        "+1 hours",
        "hour",
        "hours",
        "-1 minute",
        "+1 minute",
        "-1 minutes",
        "+1 minutes",
        "minute",
        "minutes",
        "-1 min",
        "+1 min",
        "-1 mins",
        "+1 mins",
        "min",
        "mins",
        "-1 second",
        "+1 second",
        "-1 seconds",
        "+1 seconds",
        "second",
        "seconds",
        "-1 sec",
        "+1 sec",
        "-1 secs",
        "+1 secs",
        "sec",
        "secs",
    ];
    for time in times {
        let (at, mut ucmd) = at_and_ucmd!();
        at.touch("f");
        ucmd.args(&["-d", time, "f"])
            .succeeds()
            .no_stderr()
            .no_stdout();
    }

    // From the GNU documentation:
    //
    // > The string ‘tomorrow’ is worth one day in the future
    // > (equivalent to ‘day’), the string ‘yesterday’ is worth one day
    // > in the past (equivalent to ‘day ago’).
    //
    let times = [
        "yesterday",
        "tomorrow",
        "now",
        "2 seconds",
        "2 years 1 week",
        "2 days ago",
        "2 months and 1 second",
    ];
    for time in times {
        let (at, mut ucmd) = at_and_ucmd!();
        at.touch("f");
        ucmd.args(&["-d", time, "f"])
            .succeeds()
            .no_stderr()
            .no_stdout();
    }
    let (at, mut ucmd) = at_and_ucmd!();
    at.touch("f");
    ucmd.args(&["-d", "a", "f"])
        .fails()
        .stderr_contains("touch: Unable to parse date");
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

#[test]
#[cfg(unix)]
fn test_touch_mtime_dst_fails() {
    let (_at, mut ucmd) = at_and_ucmd!();
    let file = "test_touch_set_mtime_dst_fails";

    // Some timezones use daylight savings time, this leads to problems if the
    // specified time is within the jump forward. In EST (UTC-5), there is a
    // jump from 1:59AM to 3:00AM on, March 8 2020, so any thing in-between is
    // invalid.
    // See https://www.gnu.org/software/libc/manual/html_node/TZ-Variable.html
    // for information on the TZ variable, which where the string is copied from.
    ucmd.env("TZ", "EST+5EDT,M3.2.0/2,M11.1.0/2")
        .args(&["-m", "-t", "202003080200", file])
        .fails();
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
#[cfg(not(target_os = "windows"))]
fn test_touch_trailing_slash() {
    let (_at, mut ucmd) = at_and_ucmd!();
    let file = "no-file/";
    ucmd.args(&[file]).fails().stderr_only(format!(
        "touch: cannot touch '{file}': No such file or directory\n"
    ));
}

#[test]
#[cfg(target_os = "windows")]
fn test_touch_trailing_slash_windows() {
    let (_at, mut ucmd) = at_and_ucmd!();
    let file = "no-file/";
    ucmd.args(&[file]).fails().stderr_only(format!(
        "touch: cannot touch '{file}': The filename, directory name, or volume label syntax is incorrect.\n"
    ));
}

#[test]
fn test_touch_no_such_file_error_msg() {
    let dirname = "nonexistent";
    let filename = "file";
    let path = PathBuf::from(dirname).join(filename);
    let path_str = path.to_str().unwrap();

    new_ucmd!().arg(&path).fails().stderr_only(format!(
        "touch: cannot touch '{path_str}': No such file or directory\n"
    ));
}

#[test]
#[cfg(not(any(target_os = "freebsd", target_os = "openbsd")))]
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
        "touch: cannot touch '{}': Permission denied\n",
        &full_path
    ));
}

#[test]
fn test_touch_no_args() {
    let mut ucmd = new_ucmd!();
    ucmd.fails().no_stdout().usage_error("missing file operand");
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

#[test]
fn test_touch_leap_second() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_touch_leap_sec";

    ucmd.args(&["-t", "197001010000.60", file])
        .succeeds()
        .no_stderr();

    assert!(at.file_exists(file));

    let epoch = str_to_filetime("%Y%m%d%H%M", "197001010000");
    let (atime, mtime) = get_file_times(&at, file);
    assert_eq!(atime, mtime);
    assert_eq!(atime.unix_seconds() - epoch.unix_seconds(), 60);
    assert_eq!(mtime.unix_seconds() - epoch.unix_seconds(), 60);
}

#[test]
#[cfg(not(windows))]
// File::create doesn't support trailing separator in Windows
fn test_touch_trailing_slash_no_create() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.touch("file");
    ucmd.args(&["-c", "file/"]).fails().code_is(1);

    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["-c", "no-file/"]).succeeds();
    assert!(
        !at.file_exists("no-file") && !at.dir_exists("no-file") && !at.symlink_exists("no-file")
    );

    let (at, mut ucmd) = at_and_ucmd!();
    at.relative_symlink_file("nowhere", "dangling");
    ucmd.args(&["-c", "dangling/"]).succeeds();
    assert!(!at.file_exists("nowhere"));
    assert!(at.symlink_exists("dangling"));

    let (at, mut ucmd) = at_and_ucmd!();
    at.relative_symlink_file("loop", "loop");
    ucmd.args(&["-c", "loop/"]).fails().code_is(1);
    assert!(!at.file_exists("loop"));

    #[cfg(not(target_os = "macos"))]
    // MacOS supports trailing slash for symlinks to files
    {
        let (at, mut ucmd) = at_and_ucmd!();
        at.touch("file2");
        at.relative_symlink_file("file2", "link1");
        ucmd.args(&["-c", "link1/"]).fails().code_is(1);
        assert!(at.file_exists("file2"));
        assert!(at.symlink_exists("link1"));
    }

    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir("dir");
    ucmd.args(&["-c", "dir/"]).succeeds();

    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir("dir2");
    at.relative_symlink_dir("dir2", "link2");
    ucmd.args(&["-c", "link2/"]).succeeds();
}

#[test]
fn test_touch_no_dereference_ref_dangling() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.touch("file");
    at.relative_symlink_file("nowhere", "dangling");

    ucmd.args(&["-h", "-r", "dangling", "file"]).succeeds();
}

#[test]
fn test_touch_no_dereference_dangling() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.relative_symlink_file("nowhere", "dangling");

    ucmd.args(&["-h", "dangling"]).succeeds();
}

#[test]
#[cfg(not(target_os = "openbsd"))]
fn test_touch_dash() {
    let (_, mut ucmd) = at_and_ucmd!();

    ucmd.args(&["-h", "-"]).succeeds().no_stderr().no_stdout();
}

#[test]
fn test_touch_invalid_date_format() {
    let (_at, mut ucmd) = at_and_ucmd!();
    let file = "test_touch_invalid_date_format";

    ucmd.args(&["-m", "-t", "+1000000000000 years", file])
        .fails()
        .stderr_contains("touch: invalid date format '+1000000000000 years'");
}

#[test]
#[cfg(not(target_os = "freebsd"))]
fn test_touch_symlink_with_no_deref() {
    let (at, mut ucmd) = at_and_ucmd!();
    let target = "foo.txt";
    let symlink = "bar.txt";
    let time = FileTime::from_unix_time(123, 0);

    at.touch(target);
    at.relative_symlink_file(target, symlink);
    set_symlink_file_times(at.plus(symlink), time, time).unwrap();

    ucmd.args(&["-a", "--no-dereference", symlink]).succeeds();
    // Modification time shouldn't be set to the destination's modification time
    assert_eq!(time, get_symlink_times(&at, symlink).1);
}

#[test]
#[cfg(not(target_os = "freebsd"))]
fn test_touch_reference_symlink_with_no_deref() {
    let (at, mut ucmd) = at_and_ucmd!();
    let target = "foo.txt";
    let symlink = "bar.txt";
    let arg = "baz.txt";
    let time = FileTime::from_unix_time(123, 0);

    at.touch(target);
    at.relative_symlink_file(target, symlink);
    set_symlink_file_times(at.plus(symlink), time, time).unwrap();
    at.touch(arg);

    ucmd.args(&["--reference", symlink, "--no-dereference", arg])
        .succeeds();
    // Times should be taken from the symlink, not the destination
    assert_eq!((time, time), get_symlink_times(&at, arg));
}

#[test]
fn test_obsolete_posix_format() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.env("_POSIX2_VERSION", "199209")
        .env("POSIXLY_CORRECT", "1")
        .args(&["01010000", "11111111"])
        .succeeds()
        .no_output();
    assert!(at.file_exists("11111111"));
    assert!(!at.file_exists("01010000"));
}

#[test]
fn test_obsolete_posix_format_with_year() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.env("_POSIX2_VERSION", "199209")
        .env("POSIXLY_CORRECT", "1")
        .args(&["0101000090", "11111111"])
        .succeeds()
        .no_output();
    assert!(at.file_exists("11111111"));
    assert!(!at.file_exists("0101000090"));
}
