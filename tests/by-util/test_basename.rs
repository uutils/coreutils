// spell-checker:ignore (words) reallylongexecutable

use crate::common::util::*;
#[cfg(any(unix, target_os = "redox"))]
use std::ffi::OsStr;

#[test]
fn test_help() {
    for help_flg in &["-h", "--help"] {
        new_ucmd!()
            .arg(&help_flg)
            .succeeds()
            .no_stderr()
            .stdout_contains("USAGE:");
    }
}

#[test]
fn test_version() {
    for version_flg in &["-V", "--version"] {
        assert!(new_ucmd!()
            .arg(&version_flg)
            .succeeds()
            .no_stderr()
            .stdout_str()
            .starts_with("basename"));
    }
}

#[test]
fn test_directory() {
    new_ucmd!()
        .args(&["/root/alpha/beta/gamma/delta/epsilon/omega/"])
        .succeeds()
        .stdout_only("omega\n");
}

#[test]
fn test_file() {
    new_ucmd!()
        .args(&["/etc/passwd"])
        .succeeds()
        .stdout_only("passwd\n");
}

#[test]
fn test_remove_suffix() {
    new_ucmd!()
        .args(&["/usr/local/bin/reallylongexecutable.exe", ".exe"])
        .succeeds()
        .stdout_only("reallylongexecutable\n");
}

#[test]
fn test_do_not_remove_suffix() {
    new_ucmd!()
        .args(&["/foo/bar/baz", "baz"])
        .succeeds()
        .stdout_only("baz\n");
}

#[test]
fn test_multiple_param() {
    for &multiple_param in &["-a", "--multiple"] {
        let path = "/foo/bar/baz";
        new_ucmd!()
            .args(&[multiple_param, path, path])
            .succeeds()
            .stdout_only("baz\nbaz\n"); // spell-checker:disable-line
    }
}

#[test]
fn test_suffix_param() {
    for &suffix_param in &["-s", "--suffix"] {
        let path = "/foo/bar/baz.exe";
        new_ucmd!()
            .args(&[suffix_param, ".exe", path, path])
            .succeeds()
            .stdout_only("baz\nbaz\n"); // spell-checker:disable-line
    }
}

#[test]
fn test_zero_param() {
    for &zero_param in &["-z", "--zero"] {
        let path = "/foo/bar/baz";
        new_ucmd!()
            .args(&[zero_param, "-a", path, path])
            .succeeds()
            .stdout_only("baz\0baz\0");
    }
}

fn expect_error(input: Vec<&str>) {
    assert!(!new_ucmd!()
        .args(&input)
        .fails()
        .no_stdout()
        .stderr_str()
        .is_empty());
}

#[test]
fn test_invalid_option() {
    let path = "/foo/bar/baz";
    expect_error(vec!["-q", path]);
}

#[test]
fn test_no_args() {
    expect_error(vec![]);
}

#[test]
fn test_no_args_output() {
    new_ucmd!()
        .fails()
        .stderr_is("basename: missing operand\nTry 'basename --help' for more information.");
}

#[test]
fn test_too_many_args() {
    expect_error(vec!["a", "b", "c"]);
}

#[test]
fn test_too_many_args_output() {
    new_ucmd!()
        .args(&["a", "b", "c"])
        .fails()
        .stderr_is("basename: extra operand 'c'\nTry 'basename --help' for more information.");
}

#[cfg(any(unix, target_os = "redox"))]
fn test_invalid_utf8_args(os_str: &OsStr) {
    let test_vec = vec![os_str.to_os_string()];
    new_ucmd!().args(&test_vec).succeeds().stdout_is("fo�o\n");
}

#[cfg(any(unix, target_os = "redox"))]
#[test]
fn invalid_utf8_args_unix() {
    use std::os::unix::ffi::OsStrExt;

    let source = [0x66, 0x6f, 0x80, 0x6f];
    let os_str = OsStr::from_bytes(&source[..]);
    test_invalid_utf8_args(os_str);
}
