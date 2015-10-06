use std::process::Command;
use util::*;

static PROGNAME: &'static str = "./head";
static INPUT: &'static str = "lorem_ipsum.txt";

#[path = "common/util.rs"]
#[macro_use]
mod util;

#[test]
fn test_stdin_default() {
    let mut cmd = Command::new(PROGNAME);
    let result = run_piped_stdin(&mut cmd, get_file_contents(INPUT));
    assert_eq!(result.stdout,
               get_file_contents("lorem_ipsum_default.expected"));
}

#[test]
fn test_stdin_1_line_obsolete() {
    let mut cmd = Command::new(PROGNAME);
    let result = run_piped_stdin(&mut cmd.args(&["-1"]), get_file_contents(INPUT));
    assert_eq!(result.stdout,
               get_file_contents("lorem_ipsum_1_line.expected"));
}

#[test]
fn test_stdin_1_line() {
    let mut cmd = Command::new(PROGNAME);
    let result = run_piped_stdin(&mut cmd.args(&["-n", "1"]),
                                 get_file_contents(INPUT));
    assert_eq!(result.stdout,
               get_file_contents("lorem_ipsum_1_line.expected"));
}

#[test]
fn test_stdin_5_chars() {
    let mut cmd = Command::new(PROGNAME);
    let result = run_piped_stdin(&mut cmd.args(&["-c", "5"]),
                                 get_file_contents(INPUT));
    assert_eq!(result.stdout,
               get_file_contents("lorem_ipsum_5_chars.expected"));
}

#[test]
fn test_single_default() {
    let mut cmd = Command::new(PROGNAME);
    let result = run(&mut cmd.arg(INPUT));
    assert_eq!(result.stdout,
               get_file_contents("lorem_ipsum_default.expected"));
}

#[test]
fn test_single_1_line_obsolete() {
    let mut cmd = Command::new(PROGNAME);
    let result = run(&mut cmd.args(&["-1", INPUT]));
    assert_eq!(result.stdout,
               get_file_contents("lorem_ipsum_1_line.expected"));
}

#[test]
fn test_single_1_line() {
    let mut cmd = Command::new(PROGNAME);
    let result = run(&mut cmd.args(&["-n", "1", INPUT]));
    assert_eq!(result.stdout,
               get_file_contents("lorem_ipsum_1_line.expected"));
}

#[test]
fn test_single_5_chars() {
    let mut cmd = Command::new(PROGNAME);
    let result = run(&mut cmd.args(&["-c", "5", INPUT]));
    assert_eq!(result.stdout,
               get_file_contents("lorem_ipsum_5_chars.expected"));
}

#[test]
fn test_verbose() {
    let mut cmd = Command::new(PROGNAME);
    let result = run(&mut cmd.args(&["-v", INPUT]));
    assert_eq!(result.stdout,
               get_file_contents("lorem_ipsum_verbose.expected"));
}
