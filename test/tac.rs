use std::process::Command;
use util::*;

static PROGNAME: &'static str = "./tac";

#[path = "common/util.rs"]
#[macro_use]
mod util;

#[test]
fn test_stdin_default() {
    let mut cmd = Command::new(PROGNAME);
    let result = run_piped_stdin(&mut cmd, "100\n200\n300\n400\n500");
    assert_eq!(result.stdout, "500400\n300\n200\n100\n");
}

#[test]
fn test_stdin_non_newline_separator() {
    let mut cmd = Command::new(PROGNAME);
    let result = run_piped_stdin(&mut cmd.args(&["-s", ":"]), "100:200:300:400:500");
    assert_eq!(result.stdout, "500400:300:200:100:");
}

#[test]
fn test_stdin_non_newline_separator_before() {
    let mut cmd = Command::new(PROGNAME);
    let result = run_piped_stdin(&mut cmd.args(&["-b", "-s", ":"]), "100:200:300:400:500");
    assert_eq!(result.stdout, "500:400:300:200:100");
}

#[test]
fn test_single_default() {
    let mut cmd = Command::new(PROGNAME);
    let result = run(&mut cmd.arg("prime_per_line.txt"));
    assert_eq!(result.stdout, get_file_contents("prime_per_line.expected"));
}

#[test]
fn test_single_non_newline_separator() {
    let mut cmd = Command::new(PROGNAME);
    let result = run(&mut cmd.args(&["-s", ":", "delimited_primes.txt"]));
    assert_eq!(result.stdout, get_file_contents("delimited_primes.expected"));
}

#[test]
fn test_single_non_newline_separator_before() {
    let mut cmd = Command::new(PROGNAME);
    let result = run(&mut cmd.args(&["-b", "-s", ":", "delimited_primes.txt"]));
    assert_eq!(result.stdout, get_file_contents("delimited_primes_before.expected"));
}
