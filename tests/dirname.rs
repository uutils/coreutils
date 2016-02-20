#[macro_use]
mod common;

use common::util::*;

static UTIL_NAME: &'static str = "dirname";

#[cfg(windows)]
static FS_ROOT_EXAMPLE: &'static str = "a:\\";
#[cfg(not(windows))]
static FS_ROOT_EXAMPLE: &'static str = "/";

#[cfg(windows)]
static DIRNAME_SEPARATOR: &'static str = "\\";
#[cfg(not(windows))]
static DIRNAME_SEPARATOR: &'static str = "/";

#[test]
fn test_path_with_trailing_slashes() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    let dir = format!("{0}root{1}alpha{1}beta{1}gamma{1}delta{1}epsilon{1}omega{1}{1}", FS_ROOT_EXAMPLE, DIRNAME_SEPARATOR);
    let out = ucmd.arg(dir).run().stdout;

    assert_eq!(out.trim_right(), format!("{0}root{1}alpha{1}beta{1}gamma{1}delta{1}epsilon", FS_ROOT_EXAMPLE, DIRNAME_SEPARATOR));
}

#[test]
fn test_path_without_trailing_slashes() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    let dir = format!("{0}root{1}alpha{1}beta{1}gamma{1}delta{1}epsilon{1}omega", FS_ROOT_EXAMPLE, DIRNAME_SEPARATOR);
    let out = ucmd.arg(dir).run().stdout;

    assert_eq!(out.trim_right(), format!("{0}root{1}alpha{1}beta{1}gamma{1}delta{1}epsilon", FS_ROOT_EXAMPLE, DIRNAME_SEPARATOR));
}

#[test]
fn test_root() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    let dir = FS_ROOT_EXAMPLE;
    let out = ucmd.arg(dir).run().stdout;

    assert_eq!(out.trim_right(), FS_ROOT_EXAMPLE);
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
