#[macro_use]
mod common;

use common::util::*;

static UTIL_NAME: &'static str = "dirname";


#[test]
fn test_path_with_trailing_slashes() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    let dir = "/root/alpha/beta/gamma/delta/epsilon/omega//";
    let out = ucmd.arg(dir).run().stdout;

    assert_eq!(out.trim_right(), "/root/alpha/beta/gamma/delta/epsilon");
}

#[test]
fn test_path_without_trailing_slashes() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    let dir = "/root/alpha/beta/gamma/delta/epsilon/omega";
    let out = ucmd.arg(dir).run().stdout;

    assert_eq!(out.trim_right(), "/root/alpha/beta/gamma/delta/epsilon");
}

#[test]
fn test_root() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    let dir = "/";
    let out = ucmd.arg(dir).run().stdout;

    assert_eq!(out.trim_right(), "/");
}

#[test]
fn test_pwd() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    let dir = ".";
    let out = ucmd.arg(dir).run().stdout;

    assert_eq!(out.trim_right(), ".");
}

#[test]
fn test_empty() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    let dir = "";
    let out = ucmd.arg(dir).run().stdout;

    assert_eq!(out.trim_right(), ".");
}
