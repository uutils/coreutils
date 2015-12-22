#[macro_use]
mod common;

use common::util::*;

static UTIL_NAME: &'static str = "expr";

#[test]
fn test_simple_arithmetic() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    let out = ucmd.args(&["1", "+", "1"]).run().stdout;
    assert_eq!(out, "2\n");

    let (_, mut ucmd) = testing(UTIL_NAME);
    let out = ucmd.args(&["1", "-", "1"]).run().stdout;
    assert_eq!(out, "0\n");

    let (_, mut ucmd) = testing(UTIL_NAME);
    let out = ucmd.args(&["3", "*", "2"]).run().stdout;
    assert_eq!(out, "6\n");

    let (_, mut ucmd) = testing(UTIL_NAME);
    let out = ucmd.args(&["4", "/", "2"]).run().stdout;
    assert_eq!(out, "2\n");
}

#[test]
fn test_parenthesis() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    let out = ucmd.args(&["(", "1", "+", "1", ")", "*", "2"]).run().stdout;
    assert_eq!(out, "4\n");
}

#[test]
fn test_or() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    let out = ucmd.args(&["0", "|", "foo"]).run().stdout;
    assert_eq!(out, "foo\n");

    let (_, mut ucmd) = testing(UTIL_NAME);
    let out = ucmd.args(&["foo", "|", "bar"]).run().stdout;
    assert_eq!(out, "foo\n");
}

#[test]
fn test_and() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    let out = ucmd.args(&["foo", "&", "1"]).run().stdout;
    assert_eq!(out, "foo\n");

    let (_, mut ucmd) = testing(UTIL_NAME);
    let out = ucmd.args(&["", "&", "1"]).run().stdout;
    assert_eq!(out, "0\n");
}
