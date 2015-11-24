#[macro_use]
mod common;

use common::util::*;

static UTIL_NAME: &'static str = "basename";

#[test]
fn test_directory() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    let dir = "/root/alpha/beta/gamma/delta/epsilon/omega/";
    ucmd.arg(dir);

    assert_eq!(ucmd.run().stdout.trim_right(), "omega");
}

#[test]
fn test_file() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    let file = "/etc/passwd";
    ucmd.arg(file);

    assert_eq!(ucmd.run().stdout.trim_right(), "passwd");
}

#[test]
fn test_remove_suffix() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    let path = "/usr/local/bin/reallylongexecutable.exe";
    ucmd.arg(path)
        .arg(".exe");

    assert_eq!(ucmd.run().stdout.trim_right(), "reallylongexecutable");
}

#[test]
fn test_dont_remove_suffix() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    let path = "/foo/bar/baz";
    ucmd.arg(path)
        .arg("baz");

    assert_eq!(ucmd.run().stdout.trim_right(), "baz");
}
