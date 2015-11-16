#[macro_use]
mod common;

use common::util::*;

static UTIL_NAME: &'static str = "cksum";

#[test]
fn test_single_file() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let result = ucmd.arg("lorem_ipsum.txt").run();

    assert_empty_stderr!(result);
    assert!(result.success);
    assert_eq!(result.stdout, at.read("single_file.expected"));
}

#[test]
fn test_multiple_files() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let result = ucmd.arg("lorem_ipsum.txt")
                     .arg("alice_in_wonderland.txt")
                     .run();

    assert_empty_stderr!(result);
    assert!(result.success);
    assert_eq!(result.stdout, at.read("multiple_files.expected"));
}

#[test]
fn test_stdin() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let input = at.read("lorem_ipsum.txt");
    let result = ucmd.run_piped_stdin(input);

    assert_empty_stderr!(result);
    assert!(result.success);
    assert_eq!(result.stdout, at.read("stdin.expected"));
}
