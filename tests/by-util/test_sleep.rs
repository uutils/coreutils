// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use rstest::rstest;

// spell-checker:ignore dont SIGBUS SIGSEGV sigsegv sigbus
use uutests::new_ucmd;
use uutests::util::TestScenario;
use uutests::util_name;

#[cfg(unix)]
use nix::sys::signal::Signal::{SIGBUS, SIGSEGV};
use std::io::ErrorKind;
use std::time::{Duration, Instant};

#[test]
fn test_invalid_time_interval() {
    new_ucmd!()
        .arg("xyz")
        .fails()
        .usage_error("invalid time interval 'xyz': Invalid input: xyz");
    new_ucmd!()
        .args(&["--", "-1"])
        .fails()
        .usage_error("invalid time interval '-1': Number was negative");
}

#[test]
fn test_sleep_no_suffix() {
    let millis_100 = Duration::from_millis(100);
    let before_test = Instant::now();

    new_ucmd!().args(&["0.1"]).succeeds().stdout_only("");

    let duration = before_test.elapsed();
    assert!(duration >= millis_100);
}

#[test]
fn test_sleep_s_suffix() {
    let millis_100 = Duration::from_millis(100);
    let before_test = Instant::now();

    new_ucmd!().args(&["0.1s"]).succeeds().stdout_only("");

    let duration = before_test.elapsed();
    assert!(duration >= millis_100);
}

#[test]
fn test_sleep_m_suffix() {
    let millis_600 = Duration::from_millis(600);
    let before_test = Instant::now();

    new_ucmd!().args(&["0.01m"]).succeeds().stdout_only("");

    let duration = before_test.elapsed();
    assert!(duration >= millis_600);
}

#[test]
fn test_sleep_h_suffix() {
    let millis_360 = Duration::from_millis(360);
    let before_test = Instant::now();

    new_ucmd!().args(&["0.0001h"]).succeeds().stdout_only("");

    let duration = before_test.elapsed();
    assert!(duration >= millis_360);
}

#[test]
fn test_sleep_negative_duration() {
    new_ucmd!().args(&["-1"]).fails();
    new_ucmd!().args(&["-1s"]).fails();
    new_ucmd!().args(&["-1m"]).fails();
    new_ucmd!().args(&["-1h"]).fails();
    new_ucmd!().args(&["-1d"]).fails();
}

#[test]
fn test_sleep_zero_duration() {
    new_ucmd!().args(&["0"]).succeeds().stdout_only("");
    new_ucmd!().args(&["0s"]).succeeds().stdout_only("");
    new_ucmd!().args(&["0m"]).succeeds().stdout_only("");
    new_ucmd!().args(&["0h"]).succeeds().stdout_only("");
    new_ucmd!().args(&["0d"]).succeeds().stdout_only("");
}

#[test]
fn test_sleep_no_argument() {
    new_ucmd!().fails().usage_error("missing operand");
}

#[test]
fn test_sleep_sum_duration_same_suffix() {
    let millis_200 = Duration::from_millis(100 + 100);
    let before_test = Instant::now();

    new_ucmd!()
        .args(&["0.1s", "0.1s"])
        .succeeds()
        .stdout_only("");

    let duration = before_test.elapsed();
    assert!(duration >= millis_200);
}

#[test]
fn test_sleep_sum_duration_different_suffix() {
    let millis_700 = Duration::from_millis(100 + 600);
    let before_test = Instant::now();

    new_ucmd!()
        .args(&["0.1s", "0.01m"])
        .succeeds()
        .stdout_only("");

    let duration = before_test.elapsed();
    assert!(duration >= millis_700);
}

#[test]
fn test_sleep_sum_duration_many() {
    let millis_900 = Duration::from_millis(100 + 100 + 300 + 400);
    let before_test = Instant::now();

    new_ucmd!()
        .args(&["0.1s", "0.1s", "0.3s", "0.4s"])
        .succeeds()
        .stdout_only("");

    let duration = before_test.elapsed();
    assert!(duration >= millis_900);
}

#[test]
fn test_sleep_wrong_time() {
    new_ucmd!().args(&["0.1s", "abc"]).fails();
}

#[test]
#[cfg(unix)]
fn test_sleep_stops_after_sigsegv() {
    let mut child = new_ucmd!()
        .arg("100")
        .timeout(Duration::from_secs(10))
        .run_no_wait();

    child
        .delay(100)
        .kill_with_custom_signal(SIGSEGV)
        .make_assertion()
        .with_current_output()
        .signal_is(SIGSEGV as i32) // make sure it was us who terminated the process
        .no_output();
}

#[test]
#[cfg(unix)]
fn test_sleep_stops_after_sigbus() {
    let mut child = new_ucmd!()
        .arg("100")
        .timeout(Duration::from_secs(10))
        .run_no_wait();

    child
        .delay(100)
        .kill_with_custom_signal(SIGBUS)
        .make_assertion()
        .with_current_output()
        .signal_is(SIGBUS as i32) // make sure it was us who terminated the process
        .no_output();
}

#[test]
fn test_sleep_when_single_input_exceeds_max_duration_then_no_error() {
    let mut child = new_ucmd!()
        .arg(format!("{}", u128::from(u64::MAX) + 1))
        .timeout(Duration::from_secs(10))
        .run_no_wait();

    #[cfg(unix)]
    child
        .delay(100)
        .kill()
        .make_assertion()
        .with_current_output()
        .signal_is(9) // make sure it was us who terminated the process
        .no_output();
    #[cfg(windows)]
    child
        .delay(100)
        .kill()
        .make_assertion()
        .with_current_output()
        .failure()
        .no_output();
}

#[test]
fn test_sleep_when_multiple_inputs_exceed_max_duration_then_no_error() {
    let mut child = new_ucmd!()
        .arg(format!("{}", u64::MAX))
        .arg("1")
        .timeout(Duration::from_secs(10))
        .run_no_wait();

    #[cfg(unix)]
    child
        .delay(100)
        .kill()
        .make_assertion()
        .with_current_output()
        .signal_is(9) // make sure it was us who terminated the process
        .no_output();
    #[cfg(windows)]
    child
        .delay(100)
        .kill()
        .make_assertion()
        .with_current_output()
        .failure()
        .no_output();
}

#[rstest]
#[case::whitespace_prefix(" 0.1s")]
#[case::multiple_whitespace_prefix("   0.1s")]
#[case::whitespace_suffix("0.1s ")]
#[case::mixed_newlines_spaces_tabs("\n\t0.1s \n ")]
fn test_sleep_when_input_has_whitespace_then_no_error(#[case] input: &str) {
    new_ucmd!()
        .arg(input)
        .timeout(Duration::from_secs(10))
        .succeeds()
        .no_output();
}

#[rstest]
#[case::only_space(" ")]
#[case::only_tab("\t")]
#[case::only_newline("\n")]
fn test_sleep_when_input_has_only_whitespace_then_error(#[case] input: &str) {
    new_ucmd!()
        .arg(input)
        .timeout(Duration::from_secs(10))
        .fails()
        .usage_error(format!(
            "invalid time interval '{input}': Found only whitespace in input"
        ));
}

#[test]
fn test_sleep_when_multiple_input_some_with_error_then_shows_all_errors() {
    let expected = "invalid time interval 'abc': Invalid input: abc\n\
                    sleep: invalid time interval '1years': Invalid time unit: 'years' at position 2\n\
                    sleep: invalid time interval ' ': Found only whitespace in input";

    // Even if one of the arguments is valid, but the rest isn't, we should still fail and exit early.
    // So, the timeout of 10 seconds ensures we haven't executed `thread::sleep` with the only valid
    // interval of `100000.0` seconds.
    new_ucmd!()
        .args(&["abc", "100000.0", "1years", " "])
        .timeout(Duration::from_secs(10))
        .fails()
        .usage_error(expected);
}

#[test]
fn test_negative_interval() {
    new_ucmd!()
        .args(&["--", "-1"])
        .fails()
        .usage_error("invalid time interval '-1': Number was negative");
}

#[cfg(unix)]
#[test]
#[should_panic = "Program must be run first or has not finished"]
fn test_cmd_result_signal_when_still_running_then_panic() {
    let mut child = TestScenario::new("sleep").ucmd().arg("60").run_no_wait();

    child
        .make_assertion()
        .is_alive()
        .with_current_output()
        .signal();
}

#[cfg(unix)]
#[test]
fn test_cmd_result_signal_when_kill_then_signal() {
    let mut child = TestScenario::new("sleep").ucmd().arg("60").run_no_wait();

    child.kill();
    child
        .make_assertion()
        .is_not_alive()
        .with_current_output()
        .signal_is(9)
        .signal_name_is("SIGKILL")
        .signal_name_is("KILL")
        .signal_name_is("9")
        .signal()
        .expect("Signal was none");

    let result = child.wait().unwrap();
    result
        .signal_is(9)
        .signal_name_is("SIGKILL")
        .signal_name_is("KILL")
        .signal_name_is("9")
        .signal()
        .expect("Signal was none");
}

#[cfg(unix)]
#[rstest]
#[case::signal_only_part_of_name("IGKILL")] // spell-checker: disable-line
#[case::signal_just_sig("SIG")]
#[case::signal_value_too_high("100")]
#[case::signal_value_negative("-1")]
#[should_panic = "Invalid signal name or value"]
fn test_cmd_result_signal_when_invalid_signal_name_then_panic(#[case] signal_name: &str) {
    let mut child = TestScenario::new("sleep").ucmd().arg("60").run_no_wait();
    child.kill();
    let result = child.wait().unwrap();
    result.signal_name_is(signal_name);
}

#[test]
#[cfg(unix)]
fn test_cmd_result_signal_name_is_accepts_lowercase() {
    let mut child = TestScenario::new("sleep").ucmd().arg("60").run_no_wait();
    child.kill();
    let result = child.wait().unwrap();
    result.signal_name_is("sigkill");
    result.signal_name_is("kill");
}

#[test]
fn test_uchild_when_wait_and_timeout_is_reached_then_timeout_error() {
    let ts = TestScenario::new("sleep");
    let child = ts
        .ucmd()
        .timeout(Duration::from_secs(1))
        .arg("10.0")
        .run_no_wait();

    match child.wait() {
        Err(error) if error.kind() == ErrorKind::Other => {
            std::assert_eq!(error.to_string(), "wait: Timeout of '1s' reached");
        }
        Err(error) => panic!("Assertion failed: Expected error with timeout but was: {error}"),
        Ok(_) => panic!("Assertion failed: Expected timeout of `wait`."),
    }
}

#[rstest]
#[timeout(Duration::from_secs(5))]
fn test_uchild_when_kill_and_timeout_higher_than_kill_time_then_no_panic() {
    let ts = TestScenario::new("sleep");
    let mut child = ts
        .ucmd()
        .timeout(Duration::from_secs(60))
        .arg("20.0")
        .run_no_wait();

    child.kill().make_assertion().is_not_alive();
}

#[test]
fn test_uchild_when_try_kill_and_timeout_is_reached_then_error() {
    let ts = TestScenario::new("sleep");
    let mut child = ts.ucmd().timeout(Duration::ZERO).arg("10.0").run_no_wait();

    match child.try_kill() {
        Err(error) if error.kind() == ErrorKind::Other => {
            std::assert_eq!(error.to_string(), "kill: Timeout of '0s' reached");
        }
        Err(error) => panic!("Assertion failed: Expected error with timeout but was: {error}"),
        Ok(()) => panic!("Assertion failed: Expected timeout of `try_kill`."),
    }
}

#[test]
#[should_panic = "kill: Timeout of '0s' reached"]
fn test_uchild_when_kill_with_timeout_and_timeout_is_reached_then_panic() {
    let ts = TestScenario::new("sleep");
    let mut child = ts.ucmd().timeout(Duration::ZERO).arg("10.0").run_no_wait();

    child.kill();
    panic!("Assertion failed: Expected timeout of `kill`.");
}

#[test]
#[should_panic(expected = "wait: Timeout of '1.1s' reached")]
fn test_ucommand_when_run_with_timeout_and_timeout_is_reached_then_panic() {
    let ts = TestScenario::new("sleep");
    ts.ucmd()
        .timeout(Duration::from_millis(1100))
        .arg("10.0")
        .run();

    panic!("Assertion failed: Expected timeout of `run`.")
}

#[rstest]
#[timeout(Duration::from_secs(10))]
fn test_ucommand_when_run_with_timeout_higher_then_execution_time_then_no_panic() {
    let ts = TestScenario::new("sleep");
    ts.ucmd().timeout(Duration::from_secs(60)).arg("1.0").run();
}
