#[macro_use]
mod common;

use common::util::*;

static UTIL_NAME: &'static str = "tac";

#[test]
fn test_stdin_default() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    let result = ucmd.run_piped_stdin("100\n200\n300\n400\n500");
    assert_eq!(result.stdout, "500400\n300\n200\n100\n");
}

#[test]
fn test_stdin_non_newline_separator() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    let result = ucmd.args(&["-s", ":"]).run_piped_stdin("100:200:300:400:500");
    assert_eq!(result.stdout, "500400:300:200:100:");
}

#[test]
fn test_stdin_non_newline_separator_before() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    let result = ucmd.args(&["-b", "-s", ":"]).run_piped_stdin("100:200:300:400:500");
    assert_eq!(result.stdout, "500:400:300:200:100");
}

#[test]
fn test_single_default() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let result = ucmd.arg("prime_per_line.txt").run();
    assert_eq!(result.stdout, at.read("prime_per_line.expected"));
}

#[test]
fn test_single_non_newline_separator() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let result = ucmd.args(&["-s", ":", "delimited_primes.txt"]).run();
    assert_eq!(result.stdout, at.read("delimited_primes.expected"));
}

#[test]
fn test_single_non_newline_separator_before() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let result = ucmd.args(&["-b", "-s", ":", "delimited_primes.txt"]).run();
    assert_eq!(result.stdout, at.read("delimited_primes_before.expected"));
}
