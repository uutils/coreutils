use std::process::Command;
use util::*;

static PROGNAME: &'static str = "./wc";

#[path = "common/util.rs"]
#[macro_use]
mod util;

#[test]
fn test_stdin_default() {
    let mut cmd = Command::new(PROGNAME);
    let result = run_piped_stdin(&mut cmd, get_file_contents("lorem_ipsum.txt"));
    assert_eq!(result.stdout, "  13 109 772\n");
}

#[test]
fn test_stdin_only_bytes() {
    let mut cmd = Command::new(PROGNAME);
    let result = run_piped_stdin(&mut cmd.args(&["-c"]), get_file_contents("lorem_ipsum.txt"));
    assert_eq!(result.stdout, " 772\n");
}

#[test]
fn test_stdin_all_counts() {
    let mut cmd = Command::new(PROGNAME);
    let result = run_piped_stdin(&mut cmd.args(&["-c", "-m", "-l", "-L", "-w"]), get_file_contents("alice_in_wonderland.txt"));
    assert_eq!(result.stdout, "   5  57 302 302  66\n");
}

#[test]
fn test_single_default() {
    let mut cmd = Command::new(PROGNAME);
    let result = run(&mut cmd.arg("moby_dick.txt"));
    assert_eq!(result.stdout, "   18  204 1115 moby_dick.txt\n");
}

#[test]
fn test_single_only_lines() {
    let mut cmd = Command::new(PROGNAME);
    let result = run(&mut cmd.args(&["-l", "moby_dick.txt"]));
    assert_eq!(result.stdout, "   18 moby_dick.txt\n");
}

#[test]
fn test_single_all_counts() {
    let mut cmd = Command::new(PROGNAME);
    let result = run(&mut cmd.args(&["-c", "-l", "-L", "-m", "-w", "alice_in_wonderland.txt"]));
    assert_eq!(result.stdout, "   5  57 302 302  66 alice_in_wonderland.txt\n");
}

#[test]
fn test_multiple_default() {
    let mut cmd = Command::new(PROGNAME);
    let result = run(&mut cmd.args(&["lorem_ipsum.txt", "moby_dick.txt", "alice_in_wonderland.txt"]));
    assert_eq!(result.stdout, "   13  109  772 lorem_ipsum.txt\n   18  204 1115 moby_dick.txt\n    5   57  302 alice_in_wonderland.txt\n   36  370 2189 total\n");
}
