// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore getpriority setpriority

#[cfg(any(unix, windows))]
use uutests::new_ucmd;
#[cfg(windows)]
use uutests::util::get_tests_binary;

#[test]
#[cfg(unix)]
#[cfg(not(target_os = "android"))]
fn test_get_current_niceness() {
    // Test that the nice command with no arguments returns the default nice value
    let nice = rustix::process::getpriority_process(None).unwrap();
    new_ucmd!().succeeds().stdout_is(format!("{nice}\n"));
}

#[test]
#[cfg(unix)]
#[cfg(not(target_os = "android"))]
fn test_nice_adj_negative() {
    // This assumes the test suite is run as a normal (non-root) user, and as
    // such attempting to set a negative niceness value will be rejected by
    // the OS.  If it gets denied, then we know a negative value was parsed
    // correctly.

    new_ucmd!()
        .args(&["--adj", "-20", "true"])
        .succeeds()
        .stderr_is("nice: warning: setpriority: Permission denied\n");
}

#[test]
#[cfg(unix)]
fn test_adjustment_with_no_command_should_error() {
    new_ucmd!()
        .args(&["-n", "19"])
        .fails()
        .usage_error("A command must be given with an adjustment.");
}

#[test]
#[cfg(unix)]
fn test_command_with_no_adjustment() {
    new_ucmd!().args(&["echo", "a"]).succeeds().stdout_is("a\n");
}

#[test]
#[cfg(unix)]
fn test_command_with_no_args() {
    new_ucmd!()
        .args(&["-n", "19", "echo"])
        .succeeds()
        .stdout_is("\n");
}

#[test]
#[cfg(unix)]
fn test_command_with_args() {
    new_ucmd!()
        .args(&["-n", "19", "echo", "a", "b", "c"])
        .succeeds()
        .stdout_is("a b c\n");
}

#[test]
#[cfg(unix)]
fn test_command_where_command_takes_n_flag() {
    new_ucmd!()
        .args(&["-n", "19", "echo", "-n", "a"])
        .succeeds()
        .stdout_is("a");
}

#[test]
#[cfg(unix)]
fn test_invalid_argument() {
    new_ucmd!().arg("--invalid").fails_with_code(125);
}

#[test]
#[cfg(unix)]
fn test_bare_adjustment() {
    new_ucmd!()
        .args(&["-1", "echo", "-n", "a"])
        .succeeds()
        .stdout_is("a");
}

#[test]
#[cfg(unix)]
fn test_trailing_empty_adjustment() {
    new_ucmd!()
        .args(&["-n", "1", "-n"])
        .fails()
        .stderr_str()
        .starts_with(
        "error: The argument '--adjustment <adjustment>' requires a value but none was supplied",
    );
}

#[test]
#[cfg(unix)]
fn test_nice_huge() {
    new_ucmd!()
        .args(&[
            "-n",
            "99999999999999999999999999999999999999999999999999999999999999999999999999999999999999999",
            "true",
        ])
        .succeeds()
        .no_stdout();
}

#[test]
#[cfg(unix)]
fn test_nice_huge_negative() {
    new_ucmd!().args(&["-n", "-9999999999", "true"]).succeeds();
    //.stderr_contains("Permission denied"); Depending on platform?
}

#[test]
#[cfg(unix)]
fn test_sign_middle() {
    new_ucmd!()
        .args(&["-n", "-2+4", "true"])
        .fails_with_code(125)
        .no_stdout()
        .stderr_contains("invalid");
}
//uu: "-2+4" is not a valid number: invalid digit found in string
//gnu: invalid adjustment `-2+4'
//Both message is fine

/// The nice values Cygwin reports for the six Windows priority classes.
#[cfg(windows)]
const CYGWIN_NICENESS_VALUES: [i32; 6] = [-20, -16, -8, 0, 8, 16];

#[test]
#[cfg(windows)]
fn test_get_current_niceness_windows() {
    let niceness: i32 = new_ucmd!()
        .succeeds()
        .stdout_str()
        .trim()
        .parse()
        .expect("nice should print the current niceness");
    assert!(
        CYGWIN_NICENESS_VALUES.contains(&niceness),
        "{niceness} is not the niceness of a Windows priority class"
    );
}

#[test]
#[cfg(windows)]
fn test_nice_reports_the_priority_class_it_set_windows() {
    // `-n` adds to the current niceness, so read it first to ask for an
    // absolute value regardless of the class the test runner is at.
    let current: i32 = new_ucmd!()
        .succeeds()
        .stdout_str()
        .trim()
        .parse()
        .expect("nice should print the current niceness");

    // Both sides of every boundary of Cygwin's table, with the niceness it
    // reports for that class. REALTIME is left out: it needs elevation.
    for (niceness, reported) in [
        (-19, -16),
        (-13, -16),
        (-12, -8),
        (-5, -8),
        (-4, 0),
        (3, 0),
        (4, 8),
        (11, 8),
        (12, 16),
        (19, 16),
    ] {
        new_ucmd!()
            .args(&[
                "-n",
                &(niceness - current).to_string(),
                get_tests_binary(),
                "nice",
            ])
            .succeeds()
            .stdout_is(format!("{reported}\n"));
    }
}

#[test]
#[cfg(windows)]
fn test_exit_status_of_command_windows() {
    new_ucmd!()
        .args(&["-n", "0", get_tests_binary(), "false"])
        .fails_with_code(1);
}

#[test]
#[cfg(windows)]
fn test_missing_command_windows() {
    new_ucmd!()
        .args(&["-n", "0", "this-command-does-not-exist"])
        .fails_with_code(127)
        .no_stdout();
}
