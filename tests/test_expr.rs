use common::util::*;

static UTIL_NAME: &'static str = "expr";

fn new_ucmd() -> UCommand {
    TestScenario::new(UTIL_NAME).ucmd()
}

#[test]
fn test_simple_arithmetic() {
    let out = new_ucmd()
        .args(&["1", "+", "1"]).run().stdout;
    assert_eq!(out, "2\n");

    let out = new_ucmd()
        .args(&["1", "-", "1"]).run().stdout;
    assert_eq!(out, "0\n");

    let out = new_ucmd()
        .args(&["3", "*", "2"]).run().stdout;
    assert_eq!(out, "6\n");

    let out = new_ucmd()
        .args(&["4", "/", "2"]).run().stdout;
    assert_eq!(out, "2\n");
}

#[test]
fn test_parenthesis() {
    let out = new_ucmd()
        .args(&["(", "1", "+", "1", ")", "*", "2"]).run().stdout;
    assert_eq!(out, "4\n");
}

#[test]
fn test_or() {
    let out = new_ucmd()
        .args(&["0", "|", "foo"]).run().stdout;
    assert_eq!(out, "foo\n");

    let out = new_ucmd()
        .args(&["foo", "|", "bar"]).run().stdout;
    assert_eq!(out, "foo\n");
}

#[test]
fn test_and() {
    let out = new_ucmd()
        .args(&["foo", "&", "1"]).run().stdout;
    assert_eq!(out, "foo\n");

    let out = new_ucmd()
        .args(&["", "&", "1"]).run().stdout;
    assert_eq!(out, "0\n");
}
