use crate::common::util::*;

#[test]
#[cfg(not(target_os = "android"))]
fn test_get_current_niceness() {
    // NOTE: this assumes the test suite is being run with a default niceness
    // of 0, which may not necessarily be true
    new_ucmd!().run().stdout_is("0\n");
}

#[test]
#[cfg(not(target_os = "android"))]
fn test_negative_adjustment() {
    // This assumes the test suite is run as a normal (non-root) user, and as
    // such attempting to set a negative niceness value will be rejected by
    // the OS.  If it gets denied, then we know a negative value was parsed
    // correctly.

    let res = new_ucmd!().args(&["-n", "-1", "true"]).run();
    assert!(res
        .stderr_str()
        .starts_with("nice: warning: setpriority: Permission denied")); // spell-checker:disable-line
}

#[test]
fn test_adjustment_with_no_command_should_error() {
    new_ucmd!()
        .args(&["-n", "19"])
        .fails()
        .usage_error("A command must be given with an adjustment.");
}

#[test]
fn test_command_with_no_adjustment() {
    new_ucmd!().args(&["echo", "a"]).run().stdout_is("a\n");
}

#[test]
fn test_command_with_no_args() {
    new_ucmd!()
        .args(&["-n", "19", "echo"])
        .run()
        .stdout_is("\n");
}

#[test]
fn test_command_with_args() {
    new_ucmd!()
        .args(&["-n", "19", "echo", "a", "b", "c"])
        .run()
        .stdout_is("a b c\n");
}

#[test]
fn test_command_where_command_takes_n_flag() {
    new_ucmd!()
        .args(&["-n", "19", "echo", "-n", "a"])
        .run()
        .stdout_is("a");
}
