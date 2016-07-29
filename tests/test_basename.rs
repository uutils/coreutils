use common::util::*;

static UTIL_NAME: &'static str = "basename";
fn new_ucmd() -> UCommand {
    TestScenario::new(UTIL_NAME).ucmd()
}

fn expect_successful_stdout(input: Vec<&str>, expected: &str) {
    let results = new_ucmd()
        .args(&input).run();
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

fn expect_error(input: Vec<&str>, expected_stdout: &str) {
    let results = new_ucmd()
        .args(&input).run();
    assert!(!results.success);
    assert!(results.stderr.len() > 0);
    assert_eq!(expected_stdout, results.stdout.trim_right());
}

#[cfg_attr(not(feature="test_unimplemented"),ignore)]
#[test]
fn test_multiple_param() {
    for multiple_param in vec!["-a", "--multiple"] {
        let path = "/foo/bar/baz";
        expect_successful_stdout(vec![multiple_param, path, path], "baz\nbaz");
    }
}

#[cfg_attr(not(feature="test_unimplemented"),ignore)]
#[test]
fn test_suffix_param() {
    for suffix_param in vec!["-s", "--suffix"] {
        let path = "/foo/bar/baz.exe";
        let suffix = ".exe";
        expect_successful_stdout(
            vec![suffix_param, suffix, path, path],
            "baz\nbaz"
        );
    }
}

#[cfg_attr(not(feature="test_unimplemented"),ignore)]
#[test]
fn test_zero_param() {
    for zero_param in vec!["-z", "--zero"] {
        let path = "/foo/bar/baz";
        expect_successful_stdout(vec![zero_param, "-a", path, path], "baz\0baz\0");
    }
}

#[test]
fn test_invalid_option() {
    let path = "/foo/bar/baz";
    expect_error(vec!["-q", path], "");
}

#[test]
fn test_no_args() {
    expect_error(vec![], "");
}

#[test]
fn test_too_many_args() {
    expect_error(vec!["a", "b", "c"], "");
}
