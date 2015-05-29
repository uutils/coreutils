use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::str::from_utf8;

static PROGNAME: &'static str = "./base64";

struct CmdResult {
    success: bool,
    stdout: String,
    stderr: String,
}

fn run_piped_stdin(cmd: &mut Command, input: &[u8])-> CmdResult {
    let mut command = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    command.stdin
        .take()
        .unwrap_or_else(|| panic!("Could not take child process stdin"))
        .write_all(input)
        .unwrap_or_else(|e| panic!("{}", e));

    let prog = command.wait_with_output().unwrap();
    CmdResult {
        success: prog.status.success(),
        stdout: from_utf8(&prog.stdout).unwrap().to_string(),
        stderr: from_utf8(&prog.stderr).unwrap().to_string(),
    }
}

macro_rules! assert_empty_stderr(
    ($cond:expr) => (
        if $cond.stderr.len() > 0 {
            panic!(format!("stderr: {}", $cond.stderr))
        }
    );
);

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
    assert_eq!(result.stderr, "base64: error: invalid character (Invalid character '0' at position 20)\n");
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
    assert_eq!(result.stdout, "VGhlIHF1aWNrIGJyb3du\nIGZveCBqdW1wcyBvdmVy\nIHRoZSBsYXp5IGRvZy4=\n");
}
