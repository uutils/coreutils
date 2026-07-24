// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore dont SIGBREAK

use rstest::rstest;
use std::time::Duration;
use uucore::display::Quotable;
use uutests::util::TestScenario;
use uutests::{new_ucmd, util_name};

/// A scenario plus the path to the multicall test binary, used to run child
/// commands (`sleep`, `true`, ...) portably: it exists on every test platform,
/// unlike `sh`/`sleep` from `PATH`.
fn scenario_with_bin() -> (TestScenario, String) {
    let ts = TestScenario::new(util_name!());
    let bin = ts.bin_path.to_string_lossy().into_owned();
    (ts, bin)
}

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails_with_code(125);
}

#[test]
fn test_subcommand_return_code() {
    let (ts, bin) = scenario_with_bin();
    ts.ucmd().args(&["1", &bin, "true"]).succeeds();

    ts.ucmd().args(&["1", &bin, "false"]).fails_with_code(1);
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
    let (ts, bin) = scenario_with_bin();
    ts.ucmd()
        .args(&["1700", &bin, "echo", "-n", "abcd"])
        .succeeds()
        .stdout_only("abcd");
}

#[test]
fn test_verbose() {
    let (ts, bin) = scenario_with_bin();
    for verbose_flag in ["-v", "--verbose"] {
        ts.ucmd()
            .args(&[verbose_flag, ".1", &bin, "sleep", "1"])
            .fails()
            .no_stdout()
            .stderr_contains("timeout: sending signal TERM to command");
        ts.ucmd()
            .args(&[verbose_flag, "-s0", "-k.1", ".1", &bin, "sleep", "1"])
            .fails()
            .no_stdout()
            .stderr_contains("timeout: sending signal 0 to command")
            .stderr_contains("timeout: sending signal KILL to command");
    }
}

#[test]
fn test_zero_timeout() {
    let (ts, bin) = scenario_with_bin();
    ts.ucmd()
        .args(&["-v", "0", &bin, "sleep", ".1"])
        .succeeds()
        .no_output();
    ts.ucmd()
        .args(&["-v", "0", "-s0", "-k0", &bin, "sleep", ".1"])
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
    let (ts, bin) = scenario_with_bin();
    for arg in ["-f", "--foreground"] {
        ts.ucmd()
            .args(&[arg, ".1", &bin, "sleep", "10"])
            .fails_with_code(124)
            .no_output();
    }
}

#[test]
fn test_preserve_status() {
    let (ts, bin) = scenario_with_bin();
    for arg in ["-p", "--preserve-status"] {
        ts.ucmd()
            .args(&[arg, ".1", &bin, "sleep", "10"])
            // 128 + SIGTERM = 128 + 15
            .fails_with_code(128 + 15)
            .no_output();
    }
}

#[test]
fn test_kill_after_preserves_timeout_exit_without_preserve_status() {
    let (ts, bin) = scenario_with_bin();
    ts.ucmd()
        .args(&["-k", "1", "1", &bin, "sleep", "10"])
        .fails_with_code(124)
        .no_output();
}
#[test]
fn test_preserve_status_even_when_send_signal() {
    let (ts, bin) = scenario_with_bin();
    // When sending CONT signal, process doesn't get killed or stopped.
    // So, expected result is success and code 0.
    for cont_spelling in ["CONT", "cOnT", "SIGcont"] {
        ts.ucmd()
            .args(&[
                "-s",
                cont_spelling,
                "--preserve-status",
                ".1",
                &bin,
                "sleep",
                "1",
            ])
            .succeeds()
            .no_output();
    }
}

#[test]
fn test_dont_overflow() {
    let (ts, bin) = scenario_with_bin();
    ts.ucmd()
        .args(&["9223372036854775808d", &bin, "sleep", "0"])
        .succeeds()
        .no_output();
    ts.ucmd()
        .args(&["-k", "9223372036854775808d", "10", &bin, "sleep", "0"])
        .succeeds()
        .no_output();
}

#[test]
fn test_dont_underflow() {
    let (ts, bin) = scenario_with_bin();
    ts.ucmd()
        .args(&[".0000000001", &bin, "sleep", "1"])
        .fails_with_code(124)
        .no_output();
    ts.ucmd()
        .args(&["1e-100", &bin, "sleep", "1"])
        .fails_with_code(124)
        .no_output();
    // Unlike GNU coreutils, we underflow to 1ns for very short timeouts.
    // https://debbugs.gnu.org/cgi/bugreport.cgi?bug=77535
    ts.ucmd()
        .args(&["1e-18172487393827593258", &bin, "sleep", "1"])
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
    let (ts, bin) = scenario_with_bin();
    ts.ucmd()
        .args(&["--kill-after=1", "1", &bin, "sleep", "0"])
        .succeeds()
        .no_output();
}

#[test]
#[cfg(unix)]
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
    let (ts, bin) = scenario_with_bin();
    ts.ucmd()
        .args(&["0x0.1d", &bin, "sleep", "10"])
        .timeout(Duration::from_secs(1))
        .fails_with_code(124)
        .no_output();
}

#[test]
#[cfg(unix)]
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

#[test]
#[cfg(unix)]
fn test_sigchld_ignored_by_parent() {
    let ts = TestScenario::new(util_name!());
    let bin_path = ts.bin_path.to_string_lossy();
    ts.ucmd()
        .args(&[
            "10",
            "sh",
            "-c",
            &format!("trap '' CHLD; exec {bin_path} timeout 1 true"),
        ])
        .succeeds();
}

#[test]
#[cfg(unix)]
fn test_with_background_child() {
    new_ucmd!()
        .args(&[".5", "sh", "-c", "sleep .1 & sleep 2"])
        .fails_with_code(124)
        .no_stdout();
}

#[test]
#[cfg(unix)]
fn test_forward_sigint_to_child() {
    let mut cmd = new_ucmd!()
        .args(&[
            "10",
            "sh",
            "-c",
            "trap 'echo got_int; exit 42' INT; sleep 5",
        ])
        .run_no_wait();
    #[cfg(target_os = "macos")]
    cmd.delay(1000);
    #[cfg(not(target_os = "macos"))]
    cmd.delay(100);
    cmd.kill_with_custom_signal(nix::sys::signal::Signal::SIGINT);
    cmd.make_assertion()
        .is_not_alive()
        .with_current_output()
        .stdout_contains("got_int");
}

#[test]
fn test_foreground_signal0_kill_after() {
    let (ts, bin) = scenario_with_bin();
    ts.ucmd()
        .args(&["--foreground", "-s0", "-k.1", ".1", &bin, "sleep", "10"])
        .fails_with_code(137);
}

#[test]
#[cfg(any(target_os = "linux", target_os = "android"))]
fn test_realtime_signal_names() {
    // timeout should accept RTMIN and RTMAX as valid signal names
    new_ucmd!()
        .args(&["-v", "-s", "RTMAX", ".1", "sleep", "1"])
        .fails()
        .stderr_contains("sending signal RTMAX to command");
    new_ucmd!()
        .args(&["-v", "-s", "RTMIN", ".1", "sleep", "1"])
        .fails()
        .stderr_contains("sending signal RTMIN to command");
    new_ucmd!()
        .args(&["-v", "-s", "SIGRTMAX", ".1", "sleep", "1"])
        .fails()
        .stderr_contains("sending signal RTMAX to command");
}

/// The whole process tree must be timed out in non-foreground mode, not just
/// the direct child: a grandchild sequencer (an inner `cmd.exe`) would
/// otherwise survive the timeout and create the marker file.
#[test]
#[cfg(windows)]
fn test_windows_kills_process_tree() {
    let (ts, bin) = scenario_with_bin();
    let at = &ts.fixtures;
    at.write(
        "tree_grandchild.bat",
        &format!("@echo off\r\n\"{bin}\" sleep 1\r\n\"{bin}\" touch tree_marker\r\n"),
    );
    // Outer cmd is timeout's direct child; inner cmd (running the batch file)
    // is the grandchild doing the work.
    ts.ucmd()
        .args(&[".3", "cmd", "/c", "cmd", "/c", "tree_grandchild.bat"])
        .fails_with_code(124);
    // Give a surviving grandchild ample time to reach the `touch`.
    std::thread::sleep(Duration::from_millis(2500));
    assert!(
        !at.file_exists("tree_marker"),
        "grandchild survived the process-tree kill"
    );
}

/// `-s INT` is delivered as a CTRL_BREAK to the child's group (or termination
/// without a console); either way the child dies and timeout reports 124.
#[test]
#[cfg(windows)]
fn test_windows_int_signal_kills_child() {
    let (ts, bin) = scenario_with_bin();
    ts.ucmd()
        .args(&["-v", "-s", "INT", ".1", &bin, "sleep", "10"])
        .fails_with_code(124)
        .stderr_contains("sending signal INT to command");
}

#[test]
#[cfg(windows)]
fn test_windows_kill_after() {
    let (ts, bin) = scenario_with_bin();
    ts.ucmd()
        .args(&["-s0", "-k", ".2", ".2", &bin, "sleep", "10"])
        .fails_with_code(137);
}

/// Terminations use exit code 128 + N, so `--preserve-status` reports the
/// same codes as on unix.
#[test]
#[cfg(windows)]
fn test_windows_preserve_status_signal_codes() {
    let (ts, bin) = scenario_with_bin();
    ts.ucmd()
        .args(&["-p", "-s", "KILL", ".1", &bin, "sleep", "10"])
        .fails_with_code(128 + 9);
    ts.ucmd()
        .args(&["-p", "-s", "HUP", ".1", &bin, "sleep", "10"])
        .fails_with_code(128 + 1);
}

/// POSIX signal names/numbers are accepted; names unix would reject (the
/// CRT-only SIGBREAK) are rejected here too.
#[test]
#[cfg(windows)]
fn test_windows_signal_name_parsing() {
    let (ts, bin) = scenario_with_bin();
    ts.ucmd()
        .args(&["-s", "SIGBREAK", "1", &bin, "true"])
        .fails_with_code(125)
        .usage_error("'SIGBREAK': invalid signal");
    ts.ucmd()
        .args(&["-s", "USR1", "1", &bin, "true"])
        .succeeds();
    ts.ucmd().args(&["-s", "9", "1", &bin, "true"]).succeeds();
}

/// A file that exists but is not executable yields 126, like unix.
#[test]
#[cfg(windows)]
fn test_windows_command_cannot_invoke() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    at.touch("not_executable.txt");
    // The explicit path is needed: a bare filename is searched in PATH (not
    // the current directory) and would fail with 127 instead.
    ts.ucmd()
        .args(&["1", ".\\not_executable.txt"])
        .fails_with_code(126);
}
