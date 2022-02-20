// spell-checker:ignore αbcdef

use crate::common::util::*;

#[test]
fn test_simple_arithmetic() {
    new_ucmd!()
        .args(&["1", "+", "1"])
        .succeeds()
        .stdout_only("2\n");

    new_ucmd!()
        .args(&["1", "-", "1"])
        .fails()
        .status_code(1)
        .stdout_only("0\n");

    new_ucmd!()
        .args(&["3", "*", "2"])
        .succeeds()
        .stdout_only("6\n");

    new_ucmd!()
        .args(&["4", "/", "2"])
        .succeeds()
        .stdout_only("2\n");
}

#[test]
fn test_complex_arithmetic() {
    new_ucmd!()
        .args(&["9223372036854775807", "+", "9223372036854775807"])
        .succeeds()
        .stdout_only("18446744073709551614\n");

    new_ucmd!()
        .args(&[
            "92233720368547758076549841651981984981498415651",
            "%",
            "922337203685",
        ])
        .succeeds()
        .stdout_only("533691697086\n");

    new_ucmd!()
        .args(&[
            "92233720368547758076549841651981984981498415651",
            "*",
            "922337203685",
        ])
        .succeeds()
        .stdout_only("85070591730190566808700855121818604965830915152801178873935\n");

    new_ucmd!()
        .args(&[
            "92233720368547758076549841651981984981498415651",
            "-",
            "922337203685",
        ])
        .succeeds()
        .stdout_only("92233720368547758076549841651981984059161211966\n");

    new_ucmd!()
        .args(&["9", "/", "0"])
        .fails()
        .stderr_only("expr: division by zero\n");
}

#[test]
fn test_parenthesis() {
    new_ucmd!()
        .args(&["(", "1", "+", "1", ")", "*", "2"])
        .succeeds()
        .stdout_only("4\n");
}

#[test]
fn test_or() {
    new_ucmd!()
        .args(&["0", "|", "foo"])
        .succeeds()
        .stdout_only("foo\n");

    new_ucmd!()
        .args(&["foo", "|", "bar"])
        .succeeds()
        .stdout_only("foo\n");
}

#[test]
fn test_and() {
    new_ucmd!()
        .args(&["foo", "&", "1"])
        .succeeds()
        .stdout_only("foo\n");

    new_ucmd!().args(&["", "&", "1"]).run().stdout_is("0\n");
}

#[test]
fn test_length_fail() {
    new_ucmd!().args(&["length", "αbcdef", "1"]).fails();
}

#[test]
fn test_length() {
    new_ucmd!()
        .args(&["length", "abcdef"])
        .succeeds()
        .stdout_only("6\n");
}

#[test]
fn test_length_mb() {
    new_ucmd!()
        .args(&["length", "αbcdef"])
        .succeeds()
        .stdout_only("6\n");
}

#[test]
fn test_substr() {
    new_ucmd!()
        .args(&["substr", "abc", "1", "1"])
        .succeeds()
        .stdout_only("a\n");
}

#[test]
fn test_invalid_substr() {
    new_ucmd!()
        .args(&["substr", "abc", "0", "1"])
        .fails()
        .status_code(1)
        .stdout_only("\n");

    new_ucmd!()
        .args(&["substr", "abc", &(std::usize::MAX.to_string() + "0"), "1"])
        .fails()
        .status_code(1)
        .stdout_only("\n");

    new_ucmd!()
        .args(&["substr", "abc", "0", &(std::usize::MAX.to_string() + "0")])
        .fails()
        .status_code(1)
        .stdout_only("\n");
}
