#[macro_use]
mod common;

use common::util::*;

static UTIL_NAME: &'static str = "basename";

fn expect_successful_stdout(input: Vec<&str>, expected: &str) {
    let (_, mut ucmd) = testing(UTIL_NAME);
    let results = ucmd.args(&input).run();
    assert_empty_stderr!(results);
    assert!(results.success);
    assert_eq!(expected, results.stdout.trim_right());
}

#[test]
fn test_directory() {
    let dir = "/root/alpha/beta/gamma/delta/epsilon/omega/";
    expect_successful_stdout(vec![dir], "omega");
}

#[test]
fn test_file() {
    let file = "/etc/passwd";
    expect_successful_stdout(vec![file], "passwd");
}

#[test]
fn test_remove_suffix() {
    let path = "/usr/local/bin/reallylongexecutable.exe";
    expect_successful_stdout(vec![path, ".exe"], "reallylongexecutable");
}

#[test]
fn test_dont_remove_suffix() {
    let path = "/foo/bar/baz";
    expect_successful_stdout(vec![path, "baz"], "baz");
}
