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
    for decode_param in vec!["-d", "--decode"] {
        let (_, mut ucmd) = testing(UTIL_NAME);
        let input = "aGVsbG8sIHdvcmxkIQ==";
        let result = ucmd.arg(decode_param).run_piped_stdin(input.as_bytes());

        assert_empty_stderr!(result);
        assert!(result.success);
        assert_eq!(result.stdout, "hello, world!");
    }
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
    for ignore_garbage_param in vec!["-i", "--ignore-garbage"] {
        let (_, mut ucmd) = testing(UTIL_NAME);
        let input = "aGVsbG8sIHdvcmxkIQ==\0";
        let result = ucmd.arg("-d").arg(ignore_garbage_param).run_piped_stdin(input.as_bytes());
        assert_empty_stderr!(result);
        assert!(result.success);
        assert_eq!(result.stdout, "hello, world!");
    }
}

#[test]
fn test_wrap() {
    for wrap_param in vec!["-w", "--wrap"] {
        let (_, mut ucmd) = testing(UTIL_NAME);
        let input = "The quick brown fox jumps over the lazy dog.";
        let result = ucmd.arg(wrap_param).arg("20").run_piped_stdin(input.as_bytes());

        assert_empty_stderr!(result);
        assert!(result.success);
        assert_eq!(result.stdout,
                   "VGhlIHF1aWNrIGJyb3du\nIGZveCBqdW1wcyBvdmVy\nIHRoZSBsYXp5IGRvZy4=\n");
    }
}

#[test]
fn test_wrap_no_arg() {
    for wrap_param in vec!["-w", "--wrap"] {
        let (_, mut ucmd) = testing(UTIL_NAME);
        let result = ucmd.arg(wrap_param).run();

        assert!(!result.success);
        assert!(result.stdout.len() == 0);
        assert_eq!(result.stderr.trim_right(),
                   format!("base64: error: Argument to option '{}' missing.",
                           if wrap_param == "-w" { "w" } else { "wrap" }));
    }
}

#[test]
fn test_wrap_bad_arg() {
    for wrap_param in vec!["-w", "--wrap"] {
        let (_, mut ucmd) = testing(UTIL_NAME);
        let result = ucmd.arg(wrap_param).arg("b").run();

        assert!(!result.success);
        assert!(result.stdout.len() == 0);
        assert_eq!(result.stderr.trim_right(),
                   "base64: error: Argument to option 'wrap' improperly formatted: invalid digit found in string");
    }
}
