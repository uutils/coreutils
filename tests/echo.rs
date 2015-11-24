#[macro_use]
mod common;

use common::util::*;

static UTIL_NAME: &'static str = "echo";

#[test]
fn test_default() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    assert_eq!(ucmd.run().stdout, "\n");
}

#[test]
fn test_no_trailing_newline() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    ucmd.arg("-n")
        .arg("hello_world");

    assert_eq!(ucmd.run().stdout, "hello_world");
}

#[test]
fn test_enable_escapes() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    ucmd.arg("-e")
        .arg("\\\\\\t\\r");

    assert_eq!(ucmd.run().stdout, "\\\t\r\n");
}

#[test]
fn test_disable_escapes() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    ucmd.arg("-E")
        .arg("\\b\\c\\e");

    assert_eq!(ucmd.run().stdout, "\\b\\c\\e\n");
}
