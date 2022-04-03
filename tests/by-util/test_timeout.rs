// spell-checker:ignore dont
use crate::common::util::*;

// FIXME: this depends on the system having true and false in PATH
//        the best solution is probably to generate some test binaries that we can call for any
//        utility that requires executing another program (kill, for instance)
#[test]
fn test_subcommand_return_code() {
    new_ucmd!().arg("1").arg("true").succeeds();

    new_ucmd!().arg("1").arg("false").run().status_code(1);
}

#[test]
fn test_invalid_time_interval() {
    new_ucmd!()
        .args(&["xyz", "sleep", "0"])
        .fails()
        .code_is(125)
        .usage_error("invalid time interval 'xyz'");
}

#[test]
fn test_invalid_kill_after() {
    new_ucmd!()
        .args(&["-k", "xyz", "1", "sleep", "0"])
        .fails()
        .code_is(125)
        .usage_error("invalid time interval 'xyz'");
}

#[test]
fn test_command_with_args() {
    new_ucmd!()
        .args(&["1700", "echo", "-n", "abcd"])
        .succeeds()
        .stdout_only("abcd");
}

#[test]
fn test_verbose() {
    for &verbose_flag in &["-v", "--verbose"] {
        new_ucmd!()
            .args(&[verbose_flag, ".1", "sleep", "10"])
            .fails()
            .stderr_only("timeout: sending signal TERM to command 'sleep'");
        new_ucmd!()
            .args(&[verbose_flag, "-s0", "-k.1", ".1", "sleep", "10"])
            .fails()
            .stderr_only("timeout: sending signal EXIT to command 'sleep'\ntimeout: sending signal KILL to command 'sleep'");
    }
}

#[test]
fn test_zero_timeout() {
    new_ucmd!()
        .args(&["-v", "0", "sleep", ".1"])
        .succeeds()
        .no_stderr()
        .no_stdout();
    new_ucmd!()
        .args(&["-v", "0", "-s0", "-k0", "sleep", ".1"])
        .succeeds()
        .no_stderr()
        .no_stdout();
}

#[test]
fn test_command_empty_args() {
    new_ucmd!()
        .args(&["", ""])
        .fails()
        .stderr_contains("timeout: empty string");
}

#[test]
fn test_preserve_status() {
    new_ucmd!()
        .args(&["--preserve-status", ".1", "sleep", "10"])
        .fails()
        // 128 + SIGTERM = 128 + 15
        .code_is(128 + 15)
        .no_stderr()
        .no_stdout();
}

#[test]
fn test_dont_overflow() {
    new_ucmd!()
        .args(&["9223372036854775808d", "sleep", "0"])
        .succeeds()
        .code_is(0)
        .no_stderr()
        .no_stdout();
    new_ucmd!()
        .args(&["-k", "9223372036854775808d", "10", "sleep", "0"])
        .succeeds()
        .code_is(0)
        .no_stderr()
        .no_stdout();
}

#[test]
fn test_negative_interval() {
    new_ucmd!()
        .args(&["--", "-1", "sleep", "0"])
        .fails()
        .usage_error("invalid time interval '-1'");
}

#[test]
fn test_invalid_signal() {
    new_ucmd!()
        .args(&["-s", "invalid", "1", "sleep", "0"])
        .fails()
        .usage_error("'invalid': invalid signal");
}

/// Test that the long form of the `--kill-after` argument is recognized.
#[test]
fn test_kill_after_long() {
    new_ucmd!()
        .args(&["--kill-after=1", "1", "sleep", "0"])
        .succeeds()
        .no_stdout()
        .no_stderr();
}
