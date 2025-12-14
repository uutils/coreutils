use std::time::Duration;

// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore dont
use rstest::rstest;

use uucore::display::Quotable;
use uutests::new_ucmd;

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails_with_code(125);
}

#[test]
fn test_subcommand_return_code() {
    new_ucmd!().arg("1").arg("true").succeeds();

    new_ucmd!().arg("1").arg("false").fails_with_code(1);
}

#[rstest]
#[case::alphabetic("xyz")]
#[case::single_quote("'1")]
fn test_invalid_time_interval(#[case] input: &str) {
    new_ucmd!()
        .args(&[input, "sleep", "0"])
        .fails_with_code(125)
        .usage_error(format!("invalid time interval {}", input.quote()));
}

#[test]
fn test_invalid_kill_after() {
    new_ucmd!()
        .args(&["-k", "xyz", "1", "sleep", "0"])
        .fails_with_code(125)
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
    for verbose_flag in ["-v", "--verbose"] {
        new_ucmd!()
            .args(&[verbose_flag, ".1", "sleep", "1"])
            .fails()
            .stderr_only("timeout: sending signal TERM to command 'sleep'\n");
        new_ucmd!()
            .args(&[verbose_flag, "-s0", "-k.1", ".1", "sleep", "1"])
            .fails()
            .stderr_only("timeout: sending signal EXIT to command 'sleep'\ntimeout: sending signal KILL to command 'sleep'\n");
    }
}

#[test]
fn test_zero_timeout() {
    new_ucmd!()
        .args(&["-v", "0", "sleep", ".1"])
        .succeeds()
        .no_output();
    new_ucmd!()
        .args(&["-v", "0", "-s0", "-k0", "sleep", ".1"])
        .succeeds()
        .no_output();
}

#[test]
fn test_command_empty_args() {
    new_ucmd!()
        .args(&["", ""])
        .fails()
        .stderr_contains("timeout: invalid time interval ''");
}

#[test]
fn test_foreground() {
    for arg in ["-f", "--foreground"] {
        new_ucmd!()
            .args(&[arg, ".1", "sleep", "10"])
            .fails_with_code(124)
            .no_output();
    }
}

#[test]
fn test_preserve_status() {
    for arg in ["-p", "--preserve-status"] {
        new_ucmd!()
            .args(&[arg, ".1", "sleep", "10"])
            // 128 + SIGTERM = 128 + 15
            .fails_with_code(128 + 15)
            .no_output();
    }
}

#[test]
fn test_preserve_status_even_when_send_signal() {
    // When sending CONT signal, process doesn't get killed or stopped.
    // So, expected result is success and code 0.
    for cont_spelling in ["CONT", "cOnT", "SIGcont"] {
        new_ucmd!()
            .args(&["-s", cont_spelling, "--preserve-status", ".1", "sleep", "1"])
            .succeeds()
            .no_output();
    }
}

#[test]
fn test_dont_overflow() {
    new_ucmd!()
        .args(&["9223372036854775808d", "sleep", "0"])
        .succeeds()
        .no_output();
    new_ucmd!()
        .args(&["-k", "9223372036854775808d", "10", "sleep", "0"])
        .succeeds()
        .no_output();
}

#[test]
fn test_dont_underflow() {
    new_ucmd!()
        .args(&[".0000000001", "sleep", "1"])
        .fails_with_code(124)
        .no_output();
    new_ucmd!()
        .args(&["1e-100", "sleep", "1"])
        .fails_with_code(124)
        .no_output();
    // Unlike GNU coreutils, we underflow to 1ns for very short timeouts.
    // https://debbugs.gnu.org/cgi/bugreport.cgi?bug=77535
    new_ucmd!()
        .args(&["1e-18172487393827593258", "sleep", "1"])
        .fails_with_code(124)
        .no_output();
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

#[test]
fn test_invalid_multi_byte_characters() {
    new_ucmd!()
        .args(&["10€", "sleep", "0"])
        .fails()
        .usage_error("invalid time interval '10€'");
}

/// Test that the long form of the `--kill-after` argument is recognized.
#[test]
fn test_kill_after_long() {
    new_ucmd!()
        .args(&["--kill-after=1", "1", "sleep", "0"])
        .succeeds()
        .no_output();
}

#[test]
fn test_kill_subprocess() {
    new_ucmd!()
        .args(&[
            // Make sure the CI can spawn the subprocess.
            "1",
            "sh",
            "-c",
            "trap 'echo inside_trap' TERM; sleep 5",
        ])
        .fails_with_code(124)
        .stdout_contains("inside_trap");
}

#[test]
fn test_hex_timeout_ending_with_d() {
    new_ucmd!()
        .args(&["0x0.1d", "sleep", "10"])
        .timeout(Duration::from_secs(1))
        .fails_with_code(124)
        .no_output();
}

#[test]
fn test_terminate_child_on_receiving_terminate() {
    let mut timeout_cmd = new_ucmd!()
        .args(&[
            "10",
            "sh",
            "-c",
            "trap 'echo child received TERM' TERM; sleep 5",
        ])
        .run_no_wait();
    timeout_cmd.delay(100);
    timeout_cmd.kill_with_custom_signal(nix::sys::signal::Signal::SIGTERM);
    timeout_cmd
        .make_assertion()
        .is_not_alive()
        .with_current_output()
        .code_is(143)
        .stdout_contains("child received TERM");
}

#[test]
fn test_command_not_found() {
    // Test exit code 127 when command doesn't exist
    new_ucmd!()
        .args(&["1", "/this/command/definitely/does/not/exist"])
        .fails_with_code(127);
}

#[test]
fn test_command_cannot_invoke() {
    // Test exit code 126 when command exists but cannot be invoked
    // Try to execute a directory (should give permission denied or similar)
    new_ucmd!().args(&["1", "/"]).fails_with_code(126);
}
