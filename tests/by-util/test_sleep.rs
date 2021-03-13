use crate::common::util::*;

use std::time::{Duration, Instant};

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
