#[macro_use]
mod common;

use common::util::*;

static UTIL_NAME: &'static str = "readlink";

static GIBBERISH: &'static str = "supercalifragilisticexpialidocious";


#[test]
fn test_canonicalize() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let out = ucmd.arg("-f")
                  .arg(".")
                  .run()
                  .stdout;

    assert_eq!(out.trim_right(), at.root_dir());
}

#[test]
fn test_canonicalize_existing() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let out = ucmd.arg("-e")
                  .arg(".")
                  .run()
                  .stdout;

    assert_eq!(out.trim_right(), at.root_dir());
}

#[test]
fn test_canonicalize_missing() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let mut expected = at.root_dir();
    expected.push_str("/");
    expected.push_str(GIBBERISH);

    let out = ucmd.arg("-m")
                  .arg(GIBBERISH)
                  .run()
                  .stdout;

    assert_eq!(out.trim_right(), expected);
}

#[test]
fn test_long_redirection_to_current_dir() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    // Create a 256-character path to current directory
    let dir = repeat_str("./", 128);
    let out = ucmd.arg("-n")
                  .arg("-m")
                  .arg(dir)
                  .run()
                  .stdout;

    assert_eq!(out, at.root_dir());
}

#[test]
fn test_long_redirection_to_root() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    // Create a 255-character path to root
    let dir = repeat_str("../", 85);
    let out = ucmd.arg("-n")
                  .arg("-m")
                  .arg(dir)
                  .run()
                  .stdout;

    assert_eq!(out, "/");
}
