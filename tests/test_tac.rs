use common::util::*;

static UTIL_NAME: &'static str = "tac";
fn new_ucmd() -> UCommand {
    TestScenario::new(UTIL_NAME).ucmd()
}

#[test]
fn test_stdin_default() {
    let result = new_ucmd()
        .run_piped_stdin("100\n200\n300\n400\n500");
    assert_eq!(result.stdout, "500400\n300\n200\n100\n");
}

#[test]
fn test_stdin_non_newline_separator() {
    let result = new_ucmd()
        .args(&["-s", ":"]).run_piped_stdin("100:200:300:400:500");
    assert_eq!(result.stdout, "500400:300:200:100:");
}

#[test]
fn test_stdin_non_newline_separator_before() {
    let result = new_ucmd()
        .args(&["-b", "-s", ":"]).run_piped_stdin("100:200:300:400:500");
    assert_eq!(result.stdout, "500:400:300:200:100");
}

#[test]
fn test_single_default() {
    new_ucmd().arg("prime_per_line.txt")
        .run().stdout_is_fixture("prime_per_line.expected");
}

#[test]
fn test_single_non_newline_separator() {
    new_ucmd().args(&["-s", ":", "delimited_primes.txt"])
        .run().stdout_is_fixture("delimited_primes.expected");
}

#[test]
fn test_single_non_newline_separator_before() {
    new_ucmd().args(&["-b", "-s", ":", "delimited_primes.txt"])
        .run().stdout_is_fixture("delimited_primes_before.expected");
}
