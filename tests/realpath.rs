#[macro_use]
mod common;

use common::util::*;

static UTIL_NAME: &'static str = "realpath";

#[test]
fn test_current_directory() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let out = ucmd.arg(".").run().stdout;

    assert_eq!(out.trim_right(), at.root_dir());
}

#[test]
fn test_long_redirection_to_current_dir() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    // Create a 256-character path to current directory
    let dir = repeat_str("./", 128);
    let out = ucmd.arg(dir).run().stdout;

    assert_eq!(out.trim_right(), at.root_dir());
}

#[test]
fn test_long_redirection_to_root() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    // Create a 255-character path to root
    let dir = repeat_str("../", 85);
    let out = ucmd.arg(dir).run().stdout;

    assert_eq!(out.trim_right(), "/");
}
