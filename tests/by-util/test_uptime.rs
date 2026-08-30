// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//
// spell-checker:ignore utmp runlevel testusr testx boottime

#[cfg(all(target_os = "linux", target_env = "gnu"))]
use crate::utmp::{LinuxGlibcUtmpRecord, write_linux_glibc_utmp};

#[cfg(unix)]
use uutests::at_and_ucmd;
use uutests::new_ucmd;

use regex::Regex;

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails_with_code(1);
}

#[test]
#[cfg(not(target_os = "android"))]
fn test_uptime() {
    let result = new_ucmd!().succeeds();
    result.stdout_contains(" up ");
    // Don't check for users as it doesn't show in some CI
    #[cfg(unix)]
    result
        .stdout_contains("load average:")
        .stdout_does_not_contain(",  ,");

    // Windows has no load average; the line ends after the user count.
    #[cfg(windows)]
    result
        .stdout_does_not_contain("load average")
        .stdout_matches(&Regex::new(r" up .*,  \d+ users?\n$").unwrap());
}

#[test]
#[cfg(target_os = "linux")]
fn test_write_error_handling() {
    use std::fs::File;

    let dev_full =
        File::create("/dev/full").expect("Failed to open /dev/full - test must run on Linux");

    new_ucmd!()
        .set_stdout(dev_full)
        .fails()
        .code_is(1)
        .stderr_contains("No space left on device");
}

/// Checks for files without utmpx records for which boot time cannot be calculated
#[test]
#[cfg(unix)]
#[cfg(not(any(target_os = "openbsd", target_os = "freebsd", target_os = "android")))]
// Disabled for freebsd, since it doesn't use the utmpxname() sys call to change the default utmpx
// file that is accessed using getutxent()
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
#[cfg(all(unix, feature = "cp"))]
#[cfg(not(target_os = "android"))]
fn test_uptime_with_fifo() {
    use uutests::{util::TestScenario, util_name};

    // This test can go on forever in the CI in some cases, might need aborting
    // Sometimes writing to the pipe is broken
    let ts = TestScenario::new(util_name!());

    let at = &ts.fixtures;
    at.mkfifo("fifo1");

    at.write("a", "hello");
    // Creating a child process to write to the fifo
    let mut child = ts
        .ccmd("cp")
        .arg(at.plus_as_string("a"))
        .arg(at.plus_as_string("fifo1"))
        .run_no_wait();

    ts.ucmd()
        .arg("fifo1")
        .fails()
        .stderr_contains("uptime: couldn't get boot time")
        .stdout_contains("up ???? days ??:??")
        .stdout_contains("load average");

    child.kill();
}

#[test]
#[cfg(unix)]
#[cfg(not(target_os = "freebsd"))]
fn test_uptime_with_non_existent_file() {
    // Disabled for freebsd, since it doesn't use the utmpxname() sys call to change the default utmpx
    // file that is accessed using getutxent()
    new_ucmd!()
        .arg("file1")
        .fails()
        .stderr_contains("uptime: couldn't get boot time: No such file or directory")
        .stdout_contains("up ???? days ??:??");
}

#[test]
#[cfg(all(target_os = "linux", target_env = "gnu"))]
#[cfg_attr(
    target_arch = "aarch64",
    ignore = "Issue #7159 - Test not supported on ARM64 Linux"
)]
fn test_uptime_with_file_containing_valid_boot_time_utmpx_record() {
    let (at, mut ucmd) = at_and_ucmd!();
    // Regex matches for "up   00::00" ,"up 12 days  00::00", the time can be any valid time and
    // the days can be more than 1 digit or not there. This will match even if the amount of whitespace is
    // wrong between the days and the time.

    let re = Regex::new(r"up [(\d){1,} days]*\d{1,2}:\d\d").unwrap();
    let records = [
        LinuxGlibcUtmpRecord::new(uucore::utmpx::BOOT_TIME, 0, "~", "~~", "reboot", ""),
        LinuxGlibcUtmpRecord::new(
            uucore::utmpx::RUN_LVL,
            i32::try_from(std::process::id()).unwrap(),
            "~",
            "~~",
            "runlevel",
            "",
        ),
        LinuxGlibcUtmpRecord::new(
            uucore::utmpx::USER_PROCESS,
            i32::try_from(std::process::id()).unwrap(),
            ":1",
            "~~",
            "testusr",
            "",
        ),
    ];
    write_linux_glibc_utmp(&at.plus("testx"), &records);

    ucmd.arg("testx")
        .succeeds()
        .stdout_matches(&re)
        .stdout_contains("load average");
}

#[test]
#[cfg(unix)]
fn test_uptime_with_extra_argument() {
    new_ucmd!()
        .arg("a")
        .arg("b")
        .fails()
        .stderr_contains("unexpected value 'b'");
}

/// The utmp file operand is unix-only; any operand is rejected on Windows.
#[test]
#[cfg(windows)]
fn test_uptime_with_file_windows() {
    new_ucmd!()
        .arg("file1")
        .fails_with_code(1)
        .stderr_contains("unexpected argument");
}

/// Checks whether uptime displays the correct stderr msg when its called with a directory
#[test]
#[cfg(unix)]
fn test_uptime_with_dir() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.mkdir("dir1");

    ucmd.arg("dir1")
        .fails()
        .stderr_contains("uptime: couldn't get boot time: Is a directory")
        .stdout_contains("up ???? days ??:??");
}

#[test]
#[cfg(target_os = "openbsd")]
fn test_uptime_check_users_openbsd() {
    new_ucmd!()
        .args(&["openbsd_utmp"])
        .succeeds()
        .stdout_contains("4 users");
}

#[test]
fn test_uptime_since() {
    let re = Regex::new(r"\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}").unwrap();

    new_ucmd!().arg("--since").succeeds().stdout_matches(&re);
}

#[test]
fn test_uptime_pretty_print() {
    new_ucmd!()
        .arg("-p")
        .succeeds()
        .stdout_contains("up")
        .stdout_contains("minute");
}

/// Test uptime reliability on macOS with sysctl kern.boottime fallback.
/// This addresses intermittent failures from issue #3621 by ensuring
/// the command consistently succeeds when utmpx data is unavailable.
#[test]
#[cfg(target_vendor = "apple")]
fn test_uptime_macos_reliability() {
    // Run uptime multiple times to ensure consistent success
    // (Previously would fail intermittently when utmpx had no BOOT_TIME)
    for i in 0..5 {
        let result = new_ucmd!().succeeds();

        // Verify standard output patterns
        result
            .stdout_contains("up")
            .stdout_contains("load average:");

        // Ensure no error about retrieving system uptime
        let stderr = result.stderr_str();
        assert!(
            !stderr.contains("could not retrieve system uptime"),
            "Iteration {i}: uptime should not fail on macOS (stderr: {stderr})"
        );
    }
}

/// Test uptime --since reliability on macOS.
/// Verifies the sysctl fallback works for the --since flag.
#[test]
#[cfg(target_vendor = "apple")]
fn test_uptime_since_macos() {
    let re = Regex::new(r"\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}").unwrap();

    // Run multiple times to ensure consistency
    for i in 0..3 {
        let result = new_ucmd!().arg("--since").succeeds();

        result.stdout_matches(&re);

        // Ensure no error messages
        let stderr = result.stderr_str();
        assert!(
            stderr.is_empty(),
            "Iteration {i}: uptime --since should not produce stderr on macOS (stderr: {stderr})"
        );
    }
}

/// Test that uptime output format is consistent on macOS.
/// Ensures the sysctl fallback produces properly formatted output.
#[test]
#[cfg(target_vendor = "apple")]
fn test_uptime_macos_output_format() {
    let result = new_ucmd!().succeeds();
    let stdout = result.stdout_str();

    // Verify time is present (format: HH:MM:SS)
    let time_re = Regex::new(r"\d{2}:\d{2}:\d{2}").unwrap();
    assert!(
        time_re.is_match(stdout),
        "Output should contain time in HH:MM:SS format: {stdout}"
    );

    // Verify uptime format (either "HH:MM" or "X days HH:MM")
    assert!(
        stdout.contains(" up "),
        "Output should contain 'up': {stdout}"
    );

    // Verify load average is present
    let load_re = Regex::new(r"load average: \d+\.\d+, \d+\.\d+, \d+\.\d+").unwrap();
    assert!(
        load_re.is_match(stdout),
        "Output should contain load average: {stdout}"
    );
}
