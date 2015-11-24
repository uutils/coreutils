#[macro_use]
mod common;

use common::util::*;

static UTIL_NAME: &'static str = "base64";

#[test]
fn test_encode() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    let input = "hello, world!";
    let result = ucmd.run_piped_stdin(input.as_bytes());

    assert_empty_stderr!(result);
    assert!(result.success);
    assert_eq!(result.stdout, "aGVsbG8sIHdvcmxkIQ==\n");
}

#[test]
fn test_decode() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    let input = "aGVsbG8sIHdvcmxkIQ==";
    let result = ucmd.arg("-d").run_piped_stdin(input.as_bytes());

    assert_empty_stderr!(result);
    assert!(result.success);
    assert_eq!(result.stdout, "hello, world!");
}

#[test]
fn test_garbage() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    let input = "aGVsbG8sIHdvcmxkIQ==\0";
    let result = ucmd.arg("-d").run_piped_stdin(input.as_bytes());

    assert!(!result.success);
    assert_eq!(result.stderr,
               "base64: error: invalid character (Invalid character '0' at position 20)\n");
}

#[test]
fn test_ignore_garbage() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    let input = "aGVsbG8sIHdvcmxkIQ==\0";
    let result = ucmd.arg("-d").arg("-i").run_piped_stdin(input.as_bytes());

    assert_empty_stderr!(result);
    assert!(result.success);
    assert_eq!(result.stdout, "hello, world!");
}

#[test]
fn test_wrap() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    let input = "The quick brown fox jumps over the lazy dog.";
    let result = ucmd.arg("-w").arg("20").run_piped_stdin(input.as_bytes());

    assert_empty_stderr!(result);
    assert!(result.success);
    assert_eq!(result.stdout,
               "VGhlIHF1aWNrIGJyb3du\nIGZveCBqdW1wcyBvdmVy\nIHRoZSBsYXp5IGRvZy4=\n");
}
