#[macro_use]
mod common;

use common::util::*;

static UTIL_NAME: &'static str = "sum";

#[test]
fn test_bsd_single_file() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let result = ucmd.arg("lorem_ipsum.txt").run();

    assert_empty_stderr!(result);
    assert!(result.success);
    assert_eq!(result.stdout, at.read("bsd_single_file.expected"));
}

#[test]
fn test_bsd_multiple_files() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let result = ucmd.arg("lorem_ipsum.txt")
                     .arg("alice_in_wonderland.txt")
                     .run();

    assert_empty_stderr!(result);
    assert!(result.success);
    assert_eq!(result.stdout, at.read("bsd_multiple_files.expected"));
}

#[test]
fn test_bsd_stdin() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let input = at.read("lorem_ipsum.txt");
    let result = ucmd.run_piped_stdin(input);

    assert_empty_stderr!(result);
    assert!(result.success);
    assert_eq!(result.stdout, at.read("bsd_stdin.expected"));
}

#[test]
fn test_sysv_single_file() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let result = ucmd.arg("-s").arg("lorem_ipsum.txt").run();

    assert_empty_stderr!(result);
    assert!(result.success);
    assert_eq!(result.stdout, at.read("sysv_single_file.expected"));
}

#[test]
fn test_sysv_multiple_files() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let result = ucmd.arg("-s")
                     .arg("lorem_ipsum.txt")
                     .arg("alice_in_wonderland.txt")
                     .run();

    assert_empty_stderr!(result);
    assert!(result.success);
    assert_eq!(result.stdout, at.read("sysv_multiple_files.expected"));
}

#[test]
fn test_sysv_stdin() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let input = at.read("lorem_ipsum.txt");
    let result = ucmd.arg("-s").run_piped_stdin(input);

    assert_empty_stderr!(result);
    assert!(result.success);
    assert_eq!(result.stdout, at.read("sysv_stdin.expected"));
}
