#[macro_use] mod common;

use common::util;

static UTIL_NAME: &'static str = "comm";

#[test]
fn test_comm_ab_no_args() {
    let (at, mut ucmd) = util::testing(UTIL_NAME);
    let result = ucmd.args(&["a", "b"]).run();
    assert_eq!(result.stdout, at.read("ab.expected"));
}

#[test]
fn test_comm_ab_dash_one() {
    let (at, mut ucmd) = util::testing(UTIL_NAME);
    let result = ucmd.args(&["a", "b", "-1"]).run();
    assert_eq!(result.stdout, at.read("ab1.expected"));
}

#[test]
fn test_comm_ab_dash_two() {
    let (at, mut ucmd) = util::testing(UTIL_NAME);
    let result = ucmd.args(&["a", "b", "-2"]).run();
    assert_eq!(result.stdout, at.read("ab2.expected"));
}

#[test]
fn test_comm_ab_dash_three() {
    let (at, mut ucmd) = util::testing(UTIL_NAME);
    let result = ucmd.args(&["a", "b", "-3"]).run();
    assert_eq!(result.stdout, at.read("ab3.expected"));
}

#[test]
fn test_comm_aempty() {
    let (at, mut ucmd) = util::testing(UTIL_NAME);
    let result = ucmd.args(&["a", "empty"]).run();
    assert_eq!(result.stdout, at.read("aempty.expected"));
}

#[test]
fn test_comm_emptyempty() {
    let (at, mut ucmd) = util::testing(UTIL_NAME);
    let result = ucmd.args(&["empty", "empty"]).run();
    assert_eq!(result.stdout, at.read("emptyempty.expected"));
}
