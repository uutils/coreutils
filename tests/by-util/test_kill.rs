// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore IAMNOTASIGNAL RTMAX RTMIN SIGRTMAX GHSA
use regex::Regex;
use std::os::unix::process::ExitStatusExt;
use std::process::{Child, Command};
#[cfg(any(target_os = "linux", target_os = "android"))]
use uucore::signals::realtime_signal_bounds;
use uutests::new_ucmd;

// A child process the tests will try to kill.
struct Target {
    child: Child,
    killed: bool,
}

impl Target {
    // Creates a target that will naturally die after some time if not killed
    // fast enough.
    // This timeout avoids hanging failing tests.
    fn new() -> Self {
        Self {
            child: Command::new("sleep")
                .arg("30")
                .spawn()
                .expect("cannot spawn target"),
            killed: false,
        }
    }

    // Waits for the target to complete and returns the signal it received if any.
    fn wait_for_signal(&mut self) -> Option<i32> {
        let sig = self.child.wait().expect("cannot wait on target").signal();
        self.killed = true;
        sig
    }

    fn pid(&self) -> u32 {
        self.child.id()
    }
}

impl Drop for Target {
    // Terminates this target to avoid littering test boxes with zombi processes
    // when a test fails after creating a target but before killing it.
    fn drop(&mut self) {
        if !self.killed {
            self.child.kill().expect("cannot kill target");
        }
    }
}

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails_with_code(1);
}

#[test]
fn test_kill_list_all_signals() {
    // Check for a few signals.  Do not try to be comprehensive.
    new_ucmd!()
        .arg("-l")
        .succeeds()
        .stdout_contains("KILL")
        .stdout_contains("TERM")
        .stdout_contains("HUP")
        .stdout_contains("EXIT");
}

#[cfg(any(target_os = "linux", target_os = "android"))]
#[test]
fn test_kill_list_contains_realtime_signals() {
    new_ucmd!()
        .arg("-l")
        .succeeds()
        .stdout_contains("RTMIN")
        .stdout_contains("RTMAX");
}

#[test]
fn test_kill_list_final_new_line() {
    let re = Regex::new("\\n$").unwrap();
    assert!(re.is_match(new_ucmd!().arg("-l").succeeds().stdout_str()));
}

#[test]
fn test_kill_list_all_signals_as_table() {
    // Check for a few signals.  Do not try to be comprehensive.
    new_ucmd!()
        .arg("-t")
        .succeeds()
        .stdout_contains("KILL")
        .stdout_contains("TERM")
        .stdout_contains("HUP")
        .stdout_contains("EXIT");
}

#[cfg(any(target_os = "linux", target_os = "android"))]
#[test]
fn test_kill_table_contains_realtime_signals() {
    new_ucmd!()
        .arg("-t")
        .succeeds()
        .stdout_contains("RTMIN")
        .stdout_contains("RTMAX");
}

#[test]
fn test_kill_table_starts_at_0() {
    new_ucmd!()
        .arg("-t")
        .succeeds()
        .stdout_matches(&Regex::new("^\\s?0\\sEXIT").unwrap());
}

#[test]
fn test_kill_table_lists_all_vertically() {
    // Check for a few signals.  Do not try to be comprehensive.
    let command = new_ucmd!().arg("-t").succeeds();
    let signals = command
        .stdout_str()
        .split('\n')
        .filter_map(|line| line.trim().split(' ').nth(1))
        .collect::<Vec<&str>>();

    assert!(signals.contains(&"KILL"));
    assert!(signals.contains(&"TERM"));
    assert!(signals.contains(&"HUP"));
    assert!(signals.contains(&"EXIT"));
}

#[test]
fn test_kill_list_one_signal_from_number() {
    new_ucmd!()
        .arg("-l")
        .arg("9")
        .succeeds()
        .stdout_only("KILL\n");
}

#[cfg(any(target_os = "linux", target_os = "android"))]
#[test]
fn test_kill_list_rtmax_from_name() {
    new_ucmd!()
        .arg("-l")
        .arg("RTMAX")
        .succeeds()
        .stdout_only(format!("{}\n", libc::SIGRTMAX()));
}

#[test]
fn test_kill_list_one_signal_from_invalid_number() {
    new_ucmd!()
        .arg("-l")
        .arg("99")
        .fails()
        .stderr_contains("'99': invalid signal");
}

#[test]
fn test_kill_list_one_signal_from_name() {
    // Use SIGKILL because it is 9 on all unixes.
    new_ucmd!()
        .arg("-l")
        .arg("KILL")
        .succeeds()
        .stdout_matches(&Regex::new("\\b9\\b").unwrap());
}

#[test]
fn test_kill_list_one_signal_ignore_case() {
    // Use SIGKILL because it is 9 on all unixes.
    new_ucmd!()
        .arg("-l")
        .arg("KiLl")
        .succeeds()
        .stdout_matches(&Regex::new("\\b9\\b").unwrap());
}

#[test]
fn test_kill_list_unknown_must_match_input_case() {
    new_ucmd!()
        .arg("-l")
        .arg("IaMnOtAsIgNaL")
        .fails()
        .stderr_contains("IaMnOtAsIgNaL");
}

#[test]
fn test_kill_list_all_vertically() {
    // Check for a few signals.  Do not try to be comprehensive.
    let command = new_ucmd!().arg("-l").succeeds();
    let signals = command.stdout_str().split('\n').collect::<Vec<&str>>();
    assert!(signals.contains(&"KILL"));
    assert!(signals.contains(&"TERM"));
    assert!(signals.contains(&"HUP"));
    assert!(signals.contains(&"EXIT"));
}

#[test]
fn test_kill_list_two_signal_from_name() {
    new_ucmd!()
        .arg("-l")
        .arg("INT")
        .arg("KILL")
        .succeeds()
        .stdout_matches(&Regex::new("\\d\n\\d").unwrap());
}

#[test]
fn test_kill_list_three_signal_first_unknown() {
    new_ucmd!()
        .arg("-l")
        .arg("IAMNOTASIGNAL")
        .arg("INT")
        .arg("KILL")
        .fails()
        .stderr_contains("'IAMNOTASIGNAL': invalid signal")
        .stdout_matches(&Regex::new("\\d\n\\d").unwrap());
}

#[test]
fn test_kill_set_bad_signal_name() {
    new_ucmd!()
        .arg("-s")
        .arg("IAMNOTASIGNAL")
        .fails()
        .stderr_contains("'IAMNOTASIGNAL': invalid signal");
}

#[test]
fn test_kill_out_of_range_signal_is_rejected_not_sent() {
    // An out-of-range signal number must be rejected up front (like GNU), not
    // fall through to be parsed as a negative PID and signalled with the
    // default SIGTERM. Regression for GHSA-3jmh-xh36-pj6v.
    for bad in ["-65", "-129"] {
        let mut target = Target::new();
        new_ucmd!()
            .arg(bad)
            .arg(format!("{}", target.pid()))
            .fails_with_code(1)
            .stderr_contains("invalid signal");
        // The target must have survived: kill it for real and confirm it was
        // the SIGKILL we just sent, not an earlier stray SIGTERM.
        target.child.kill().expect("cannot kill surviving target");
        assert_eq!(target.wait_for_signal(), Some(libc::SIGKILL));
    }
}

#[test]
fn test_kill_with_default_signal() {
    let mut target = Target::new();
    new_ucmd!().arg(format!("{}", target.pid())).succeeds();
    assert_eq!(target.wait_for_signal(), Some(libc::SIGTERM));
}

#[test]
fn test_kill_with_signal_number_old_form() {
    let mut target = Target::new();
    new_ucmd!()
        .arg("-9")
        .arg(format!("{}", target.pid()))
        .succeeds();
    assert_eq!(target.wait_for_signal(), Some(9));
}

#[test]
fn test_kill_with_signal_name_old_form() {
    for arg in ["-Kill", "-KILL"] {
        let mut target = Target::new();
        new_ucmd!()
            .arg(arg)
            .arg(format!("{}", target.pid()))
            .succeeds();
        assert_eq!(target.wait_for_signal(), Some(libc::SIGKILL));
    }
}

#[test]
fn test_kill_with_lower_case_signal_name_old_form() {
    let target = Target::new();
    new_ucmd!()
        .arg("-kill")
        .arg(format!("{}", target.pid()))
        .fails()
        .stderr_contains("unexpected argument");
}

#[test]
fn test_kill_with_signal_prefixed_name_old_form() {
    let mut target = Target::new();
    new_ucmd!()
        .arg("-SIGKILL")
        .arg(format!("{}", target.pid()))
        .succeeds();
    assert_eq!(target.wait_for_signal(), Some(libc::SIGKILL));
}

#[test]
fn test_kill_with_signal_number_new_form() {
    let mut target = Target::new();
    new_ucmd!()
        .arg("-s")
        .arg("9")
        .arg(format!("{}", target.pid()))
        .succeeds();
    assert_eq!(target.wait_for_signal(), Some(9));
}

#[test]
fn test_kill_with_signal_name_new_form() {
    let mut target = Target::new();
    new_ucmd!()
        .arg("-s")
        .arg("KILL")
        .arg(format!("{}", target.pid()))
        .succeeds();
    assert_eq!(target.wait_for_signal(), Some(libc::SIGKILL));
}

#[test]
fn test_kill_with_signal_name_new_form_ignore_case() {
    let mut target = Target::new();
    new_ucmd!()
        .arg("-s")
        .arg("KiLl")
        .arg(format!("{}", target.pid()))
        .succeeds();
    assert_eq!(target.wait_for_signal(), Some(libc::SIGKILL));
}

#[test]
fn test_kill_with_signal_prefixed_name_new_form() {
    let mut target = Target::new();
    new_ucmd!()
        .arg("-s")
        .arg("SIGKILL")
        .arg(format!("{}", target.pid()))
        .succeeds();
    assert_eq!(target.wait_for_signal(), Some(libc::SIGKILL));
}

#[test]
fn test_kill_with_signal_prefixed_name_new_form_ignore_case() {
    let mut target = Target::new();
    new_ucmd!()
        .arg("-s")
        .arg("SiGKiLl")
        .arg(format!("{}", target.pid()))
        .succeeds();
    assert_eq!(target.wait_for_signal(), Some(libc::SIGKILL));
}

#[test]
fn test_kill_with_signal_name_new_form_unknown_must_match_input_case() {
    let target = Target::new();
    new_ucmd!()
        .arg("-s")
        .arg("IaMnOtAsIgNaL")
        .arg(format!("{}", target.pid()))
        .fails()
        .stderr_contains("'IaMnOtAsIgNaL': invalid signal");
}

#[test]
fn test_kill_no_pid_provided() {
    new_ucmd!()
        .fails()
        .stderr_contains("no process ID specified");
}

#[test]
fn test_kill_with_signal_exit_new_form() {
    let target = Target::new();
    new_ucmd!()
        .arg("-s")
        .arg("EXIT")
        .arg(format!("{}", target.pid()))
        .succeeds();
}

#[test]
fn test_kill_with_signal_number_hidden_compatibility_option() {
    let mut target = Target::new();
    new_ucmd!()
        .arg("-n")
        .arg("9")
        .arg(format!("{}", target.pid()))
        .succeeds();
    assert_eq!(target.wait_for_signal(), Some(9));
}

#[test]
fn test_kill_with_signal_and_list() {
    let target = Target::new();
    new_ucmd!()
        .arg("-s")
        .arg("EXIT")
        .arg(format!("{}", target.pid()))
        .arg("-l")
        .fails();
}

#[test]
fn test_kill_with_list_lower_bits() {
    new_ucmd!()
        .arg("-l")
        .arg("128")
        .succeeds()
        .stdout_contains("EXIT");

    new_ucmd!()
        .arg("-l")
        .arg("143")
        .succeeds()
        .stdout_contains("TERM");

    new_ucmd!()
        .arg("-l")
        .arg("256")
        .succeeds()
        .stdout_contains("EXIT");

    new_ucmd!()
        .arg("-l")
        .arg("2304")
        .succeeds()
        .stdout_contains("EXIT");
}

#[test]
fn test_kill_with_list_lower_bits_unrecognized() {
    new_ucmd!().arg("-l").arg("111").fails();
    new_ucmd!().arg("-l").arg("384").fails();
}

#[cfg(any(target_os = "linux", target_os = "android"))]
#[test]
fn test_kill_with_list_unnamed_signal_numbers() {
    new_ucmd!()
        .arg("-l")
        .arg("32")
        .succeeds()
        .stdout_only("32\n");
    new_ucmd!()
        .arg("-l")
        .arg("33")
        .succeeds()
        .stdout_only("33\n");
}

#[cfg(any(target_os = "linux", target_os = "android"))]
#[test]
fn test_kill_with_list_all_signal_numbers_up_to_last_named_signal() {
    let last_signal_name = new_ucmd!()
        .arg("-l")
        .succeeds()
        .stdout_str()
        .lines()
        .last()
        .unwrap()
        .to_string();

    let last_signal_number: usize = new_ucmd!()
        .arg("-l")
        .arg("--")
        .arg(&last_signal_name)
        .succeeds()
        .stdout_str()
        .trim()
        .parse()
        .unwrap();

    let args = std::iter::once(String::from("--"))
        .chain((0..=last_signal_number).map(|signal| signal.to_string()))
        .collect::<Vec<_>>();

    new_ucmd!().arg("-l").args(&args).succeeds();
}

#[test]
fn test_kill_with_signal_and_table() {
    let target = Target::new();
    new_ucmd!()
        .arg("-s")
        .arg("EXIT")
        .arg(format!("{}", target.pid()))
        .arg("-t")
        .fails();
}

// Listing signals to a full device must report the write error and exit
// non-zero, not panic/abort. Covers -l, -l <name>, --list <number>, and --table.
#[cfg(target_os = "linux")]
#[test]
fn test_kill_list_signals_write_error_is_reported() {
    for args in [
        vec!["-l"],
        vec!["-l", "TERM"],
        vec!["--list", "9"],
        vec!["--table"],
    ] {
        new_ucmd!()
            .args(&args)
            .set_stdout(std::fs::File::create("/dev/full").unwrap())
            .fails()
            .stderr_is("kill: write error: No space left on device\n");
    }
}

/// Test that `kill -1` (signal without PID) reports "no process ID" error
/// instead of being misinterpreted as pid=-1 which would kill all processes.
/// This matches GNU kill behavior.
#[test]
fn test_kill_signal_only_no_pid() {
    // Test with -1 (SIGHUP)
    new_ucmd!()
        .arg("-1")
        .fails()
        .stderr_contains("no process ID specified");

    // Test with -9 (SIGKILL)
    new_ucmd!()
        .arg("-9")
        .fails()
        .stderr_contains("no process ID specified");

    // Test with -TERM
    new_ucmd!()
        .arg("-TERM")
        .fails()
        .stderr_contains("no process ID specified");
}

#[test]
fn test_kill_signal_zero_process() {
    let target = Target::new();
    // kill -0 should succeed for a running process (signal 0 = existence check)
    new_ucmd!()
        .arg("-0")
        .arg(format!("{}", target.pid()))
        .succeeds();
}

#[test]
fn test_kill_signal_zero_new_form() {
    let target = Target::new();
    // kill -s 0 should also work
    new_ucmd!()
        .arg("-s")
        .arg("0")
        .arg(format!("{}", target.pid()))
        .succeeds();
}

#[test]
fn test_kill_signal_zero_nonexistent() {
    // kill -0 with a nonexistent PID should fail
    new_ucmd!().arg("-0").arg("999999999").fails();
}

#[test]
fn test_kill_signal_zero_current_process_group() {
    // kill -0 0 should succeed (checks current process group)
    new_ucmd!().arg("-0").arg("0").succeeds();
}

#[cfg(any(target_os = "linux", target_os = "android"))]
#[test]
fn test_kill_realtime_signal() {
    let mut target = Target::new();
    // kill -s RTMIN should send SIGRTMIN and terminate the process
    new_ucmd!()
        .arg("-s")
        .arg("RTMIN")
        .arg(format!("{}", target.pid()))
        .succeeds();
    assert_eq!(target.wait_for_signal(), Some(libc::SIGRTMIN()));
}

#[cfg(any(target_os = "linux", target_os = "android"))]
#[test]
fn test_kill_with_rtmax_offset() {
    let (_, rtmax) = realtime_signal_bounds().unwrap();
    let sig: i32 = (rtmax as i32) - 7;

    let mut target = Target::new();
    new_ucmd!()
        .arg("-s")
        .arg("SIGRTMAX-7")
        .arg(format!("{}", target.pid()))
        .succeeds();
    assert_eq!(target.wait_for_signal(), Some(sig));
}

#[cfg(any(target_os = "linux", target_os = "android"))]
#[test]
fn test_kill_with_rtmin_offset() {
    let (rtmin, _) = realtime_signal_bounds().unwrap();
    let sig: i32 = (rtmin as i32) + 7;

    let mut target = Target::new();
    new_ucmd!()
        .arg("-s")
        .arg("SIGRTMIN+7")
        .arg(format!("{}", target.pid()))
        .succeeds();
    assert_eq!(target.wait_for_signal(), Some(sig));
}
