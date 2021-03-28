use crate::common::util::*;

#[test]
fn test_stdin_default() {
    new_ucmd!()
        .pipe_in("100\n200\n300\n400\n500")
        .run()
        .stdout_is("500400\n300\n200\n100\n");
}

#[test]
fn test_stdin_non_newline_separator() {
    new_ucmd!()
        .args(&["-s", ":"])
        .pipe_in("100:200:300:400:500")
        .run()
        .stdout_is("500400:300:200:100:");
}

#[test]
fn test_stdin_non_newline_separator_before() {
    new_ucmd!()
        .args(&["-b", "-s", ":"])
        .pipe_in("100:200:300:400:500")
        .run()
        .stdout_is("500:400:300:200:100");
}

#[test]
fn test_single_default() {
    new_ucmd!()
        .arg("prime_per_line.txt")
        .run()
        .stdout_is_fixture("prime_per_line.expected");
}

#[test]
fn test_single_non_newline_separator() {
    new_ucmd!()
        .args(&["-s", ":", "delimited_primes.txt"])
        .run()
        .stdout_is_fixture("delimited_primes.expected");
}

#[test]
fn test_single_non_newline_separator_before() {
    new_ucmd!()
        .args(&["-b", "-s", ":", "delimited_primes.txt"])
        .run()
        .stdout_is_fixture("delimited_primes_before.expected");
}

#[test]
fn test_invalid_input() {
    let (at, mut ucmd) = at_and_ucmd!();

    ucmd.arg("b")
        .fails()
        .stderr
        .contains("tac: error: failed to open 'b' for reading");

    let (at, mut ucmd) = at_and_ucmd!();

    at.mkdir("a");
    ucmd.arg("a")
        .run()
        .stderr
        .contains("tac: error: failed to read 'a'");
}
