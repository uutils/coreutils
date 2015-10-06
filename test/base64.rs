use std::process::Command;
use util::*;

static PROGNAME: &'static str = "./base64";

#[path = "common/util.rs"]
#[macro_use]
mod util;

#[test]
fn test_encode() {
    let input = "hello, world!";
    let mut cmd = Command::new(PROGNAME);
    let result = run_piped_stdin(&mut cmd, input.as_bytes());

    assert_empty_stderr!(result);
    assert!(result.success);
    assert_eq!(result.stdout, "aGVsbG8sIHdvcmxkIQ==\n");
}

#[test]
fn test_decode() {
    let input = "aGVsbG8sIHdvcmxkIQ==";
    let mut cmd = Command::new(PROGNAME);
    let result = run_piped_stdin(&mut cmd.arg("-d"), input.as_bytes());

    assert_empty_stderr!(result);
    assert!(result.success);
    assert_eq!(result.stdout, "hello, world!");
}

#[test]
fn test_garbage() {
    let input = "aGVsbG8sIHdvcmxkIQ==\0";
    let mut cmd = Command::new(PROGNAME);
    let result = run_piped_stdin(&mut cmd.arg("-d"), input.as_bytes());

    assert!(!result.success);
    assert_eq!(result.stderr,
               "base64: error: invalid character (Invalid character '0' at position 20)\n");
}

#[test]
fn test_ignore_garbage() {
    let input = "aGVsbG8sIHdvcmxkIQ==\0";
    let mut cmd = Command::new(PROGNAME);
    let result = run_piped_stdin(&mut cmd.arg("-d").arg("-i"), input.as_bytes());

    assert_empty_stderr!(result);
    assert!(result.success);
    assert_eq!(result.stdout, "hello, world!");
}

#[test]
fn test_wrap() {
    let input = "The quick brown fox jumps over the lazy dog.";
    let mut cmd = Command::new(PROGNAME);
    let result = run_piped_stdin(&mut cmd.arg("-w").arg("20"), input.as_bytes());

    assert_empty_stderr!(result);
    assert!(result.success);
    assert_eq!(result.stdout,
               "VGhlIHF1aWNrIGJyb3du\nIGZveCBqdW1wcyBvdmVy\nIHRoZSBsYXp5IGRvZy4=\n");
}
