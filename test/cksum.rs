use std::process::Command;
use util::*;

static PROGNAME: &'static str = "./cksum";

#[path = "common/util.rs"]
#[macro_use]
mod util;

#[test]
fn test_single_file() {
    let mut cmd = Command::new(PROGNAME);
    let result = run(&mut cmd.arg("lorem_ipsum.txt"));

    assert_empty_stderr!(result);
    assert!(result.success);
    assert_eq!(result.stdout, get_file_contents("single_file.expected"));
}

#[test]
fn test_multiple_files() {
    let mut cmd = Command::new(PROGNAME);
    let result = run(&mut cmd.arg("lorem_ipsum.txt").arg("alice_in_wonderland.txt"));

    assert_empty_stderr!(result);
    assert!(result.success);
    assert_eq!(result.stdout, get_file_contents("multiple_files.expected"));
}

#[test]
fn test_stdin() {
    let input = get_file_contents("lorem_ipsum.txt");
    let mut cmd = Command::new(PROGNAME);
    let result = run_piped_stdin(&mut cmd, input);

    assert_empty_stderr!(result);
    assert!(result.success);
    assert_eq!(result.stdout, get_file_contents("stdin.expected"));
}
