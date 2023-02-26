// spell-checker:ignore dont
use crate::common::util::*;

use std::time::{Duration, Instant};

#[test]
fn test_invalid_time_interval() {
    new_ucmd!()
        .arg("xyz")
        .fails()
        .usage_error("invalid time interval 'xyz'");
    new_ucmd!()
        .args(&["--", "-1"])
        .fails()
        .usage_error("invalid time interval '-1'");
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
fn test_sleep_when_single_input_exceeds_max_duration_then_no_error() {
    let mut child = new_ucmd!()
        .arg(format!("{}", u64::MAX as u128 + 1))
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

#[test]
fn test_negative_interval() {
    new_ucmd!()
        .args(&["--", "-1"])
        .fails()
        .usage_error("invalid time interval '-1'");
}
