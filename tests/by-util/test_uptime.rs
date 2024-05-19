// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use crate::common::util::TestScenario;
use regex::Regex;

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails().code_is(1);
}

#[test]
#[cfg(not(target_os = "openbsd"))]
fn test_uptime() {
    TestScenario::new(util_name!())
        .ucmd()
        .succeeds()
        .stdout_contains("load average:")
        .stdout_contains(" up ");

    // Don't check for users as it doesn't show in some CI
}

/// Checks for files without utmpx for which boot time cannot be calculated
#[test]
#[cfg(not(target_os = "openbsd"))]
fn test_uptime_for_file_without_utmpx_records() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.write("file1", "hello");

    ucmd.arg(at.plus_as_string("file1"))
        .fails()
        .stderr_contains("uptime: couldn't get boot time")
        .stdout_contains("up ???? days ??:??")
        .stdout_contains("load average");
}

/// Checks whether uptime displays the correct stderr msg when its called with a fifo
#[test]
#[cfg(target_os = "linux")]
fn test_uptime_with_fifo() {
    // This test can go on forever in the CI in some cases, might need aborting
    let ts = TestScenario::new(util_name!());

    let at = &ts.fixtures;
    at.mkfifo("fifo1");

    let child = ts.ucmd().arg("fifo1").run_no_wait();

    let _ = std::fs::write(at.plus("fifo1"), vec![0; 10]);

    child
        .wait()
        .unwrap()
        .failure()
        .stderr_contains("uptime: couldn't get boot time: Illegal seek")
        .stdout_contains("up ???? days ??:??")
        .stdout_contains("load average");
}

#[test]
#[cfg(not(target_os = "openbsd"))]
fn test_uptime_with_non_existent_file() {
    let ts = TestScenario::new(util_name!());

    ts.ucmd()
        .arg("file1")
        .fails()
        .stderr_contains("uptime: couldn't get boot time: No such file or directory")
        .stdout_contains("up ???? days ??:??");
}

#[test]
#[cfg(not(target_os = "openbsd"))]
fn test_uptime_with_file_containing_valid_utmpx_record() {
    let ts = TestScenario::new(util_name!());
    let re = Regex::new(r"up {1,2}[(\d){1,} days]*\d{1,2}:\d\d").unwrap();
    #[cfg(not(target_os = "macos"))]
    ts.ucmd()
        .arg("validRecord.txt")
        .succeeds()
        .stdout_matches(&re)
        .stdout_contains("load average");
}

/// Assuming /var/log/wtmp has multiple records, /var/log/wtmp doesn't seem to exist in macos
#[test]
#[cfg(not(target_os = "openbsd"))]
fn test_uptime_with_file_containing_multiple_valid_utmpx_record() {
    let ts = TestScenario::new(util_name!());
    // Checking for up   00:00 [can be any time]
    let re = Regex::new(r"up {1,2}[(\d){1,} days]*\d{1,2}:\d\d").unwrap();
    // Can be multiple users, for double digit users, only matches the last digit.
    let re_users = Regex::new(r"\d user[s]?").unwrap();
    ts.ucmd()
        .arg("validMultipleRecords.txt")
        .succeeds()
        .stdout_matches(&re)
        .stdout_matches(&re_users)
        .stdout_contains("load average");
}
#[test]
#[cfg(not(target_os = "openbsd"))]
fn test_uptime_with_extra_argument() {
    let ts = TestScenario::new(util_name!());

    ts.ucmd()
        .arg("a")
        .arg("b")
        .fails()
        .stderr_contains("extra operand 'b'");
}
/// Here we test if partial records are parsed properly and this may return an uptime of hours or
/// days, assuming /var/log/wtmp contains multiple records
#[test]
#[cfg(not(target_os = "openbsd"))]
fn test_uptime_with_file_containing_multiple_valid_utmpx_record_with_partial_records() {
    let ts = TestScenario::new(util_name!());

    let re_users = Regex::new(r"\d user[s]?").unwrap();
    // Regex matches for "up   00::00" ,"up 12 days  00::00", the time can be any valid time and
    // the days can be more than 1 digit or not there. This will match even if the amount of whitespace is
    // wrong between the days and the time.
    let re_uptime = Regex::new(r"up {1,2}[(\d){1,} days]*\d{1,2}:\d\d").unwrap();
    ts.ucmd()
        .arg("validMultipleRecords.txt")
        .succeeds()
        .stdout_contains("load average")
        .stdout_matches(&re_users)
        .stdout_matches(&re_uptime);
}

/// Checks whether uptime displays the correct stderr msg when its called with a directory
#[test]
#[cfg(not(target_os = "openbsd"))]
fn test_uptime_with_dir() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    at.mkdir("dir1");

    ts.ucmd()
        .arg("dir1")
        .fails()
        .stderr_contains("uptime: couldn't get boot time: Is a directory")
        .stdout_contains("up ???? days ??:??");
}

#[test]
#[cfg(not(target_os = "openbsd"))]
fn test_uptime_since() {
    let re = Regex::new(r"\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}").unwrap();

    new_ucmd!().arg("--since").succeeds().stdout_matches(&re);
}

#[test]
fn test_failed() {
    new_ucmd!().arg("will-fail").fails();
}
