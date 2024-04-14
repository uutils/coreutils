// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use crate::common::util::TestScenario;
use regex::Regex;
use std::os::unix::process::ExitStatusExt;
use std::process::{Child, Command};

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
    new_ucmd!().arg("--definitely-invalid").fails().code_is(1);
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
        .stdout_does_not_contain("EXIT");
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
        .stdout_contains("HUP");
}

#[test]
fn test_kill_table_starts_at_1() {
    new_ucmd!()
        .arg("-t")
        .succeeds()
        .stdout_matches(&Regex::new("^\\s?1\\sHUP").unwrap());
}

#[test]
fn test_kill_table_lists_all_vertically() {
    // Check for a few signals.  Do not try to be comprehensive.
    let command = new_ucmd!().arg("-t").succeeds();
    let signals = command
        .stdout_str()
        .split('\n')
        .flat_map(|line| line.trim().split(" ").nth(1))
        .collect::<Vec<&str>>();

    assert!(signals.contains(&"KILL"));
    assert!(signals.contains(&"TERM"));
    assert!(signals.contains(&"HUP"));
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
fn test_kill_list_all_vertically() {
    // Check for a few signals.  Do not try to be comprehensive.
    let command = new_ucmd!().arg("-l").succeeds();
    let signals = command.stdout_str().split('\n').collect::<Vec<&str>>();
    assert!(signals.contains(&"KILL"));
    assert!(signals.contains(&"TERM"));
    assert!(signals.contains(&"HUP"));
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
        .arg("IAMNOTASIGNAL") // spell-checker:disable-line
        .arg("INT")
        .arg("KILL")
        .fails()
        .stderr_contains("unknown signal")
        .stdout_matches(&Regex::new("\\d\n\\d").unwrap());
}

#[test]
fn test_kill_set_bad_signal_name() {
    // spell-checker:disable-line
    new_ucmd!()
        .arg("-s")
        .arg("IAMNOTASIGNAL") // spell-checker:disable-line
        .fails()
        .stderr_contains("unknown signal");
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
    let mut target = Target::new();
    new_ucmd!()
        .arg("-KILL")
        .arg(format!("{}", target.pid()))
        .succeeds();
    assert_eq!(target.wait_for_signal(), Some(libc::SIGKILL));
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
fn test_kill_no_pid_provided() {
    // spell-checker:disable-line
    new_ucmd!()
        .fails()
        .stderr_contains("no process ID specified");
}
