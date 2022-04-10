// spell-checker:ignore (words) defaultcheck nocheck

use crate::common::util::*;

#[test]
fn ab_no_args() {
    new_ucmd!()
        .args(&["a", "b"])
        .succeeds()
        .stdout_only_fixture("ab.expected");
}

#[test]
fn ab_dash_one() {
    new_ucmd!()
        .args(&["a", "b", "-1"])
        .succeeds()
        .stdout_only_fixture("ab1.expected");
}

#[test]
fn ab_dash_two() {
    new_ucmd!()
        .args(&["a", "b", "-2"])
        .succeeds()
        .stdout_only_fixture("ab2.expected");
}

#[test]
fn ab_dash_three() {
    new_ucmd!()
        .args(&["a", "b", "-3"])
        .succeeds()
        .stdout_only_fixture("ab3.expected");
}

#[test]
fn a_empty() {
    new_ucmd!()
        .args(&["a", "empty"])
        .succeeds()
        .stdout_only_fixture("aempty.expected"); // spell-checker:disable-line
}

#[test]
fn empty_empty() {
    new_ucmd!()
        .args(&["empty", "empty"])
        .succeeds()
        .stdout_only_fixture("emptyempty.expected"); // spell-checker:disable-line
}

#[cfg_attr(not(feature = "test_unimplemented"), ignore)]
#[test]
fn output_delimiter() {
    new_ucmd!()
        .args(&["--output-delimiter=word", "a", "b"])
        .succeeds()
        .stdout_only_fixture("ab_delimiter_word.expected");
}

#[cfg_attr(not(feature = "test_unimplemented"), ignore)]
#[test]
fn output_delimiter_require_arg() {
    new_ucmd!()
        .args(&["--output-delimiter=", "a", "b"])
        .fails()
        .stderr_only("error to be defined");
}

// even though (info) documentation suggests this is an option
// in latest GNU Coreutils comm, it actually is not.
// this test is essentially an alarm in case some well-intending
// developer implements it.
//marked as unimplemented as error message not set yet.
#[cfg_attr(not(feature = "test_unimplemented"), ignore)]
#[test]
fn zero_terminated() {
    for param in ["-z", "--zero-terminated"] {
        new_ucmd!()
            .args(&[param, "a", "b"])
            .fails()
            .stderr_only("error to be defined");
    }
}

#[cfg_attr(not(feature = "test_unimplemented"), ignore)]
#[test]
fn check_order() {
    new_ucmd!()
        .args(&["--check-order", "bad_order_1", "bad_order_2"])
        .fails()
        .stdout_is_fixture("bad_order12.check_order.expected")
        .stderr_is("error to be defined");
}

#[cfg_attr(not(feature = "test_unimplemented"), ignore)]
#[test]
fn nocheck_order() {
    new_ucmd!()
        .args(&["--nocheck-order", "bad_order_1", "bad_order_2"])
        .succeeds()
        .stdout_only_fixture("bad_order12.nocheck_order.expected");
}

// when neither --check-order nor --no-check-order is provided,
// stderr and the error code behaves like check order, but stdout
// behaves like nocheck_order. However with some quirks detailed below.
#[cfg_attr(not(feature = "test_unimplemented"), ignore)]
#[test]
fn defaultcheck_order() {
    new_ucmd!()
        .args(&["a", "bad_order_1"])
        .fails()
        .stderr_only("error to be defined");
}

// * the first: if both files are not in order, the default behavior is the only
// behavior that will provide an error message

// * the second: if two rows are paired but are out of order,
// it won't matter if all rows in the two files are exactly the same.
// This is specified in the documentation

#[test]
fn defaultcheck_order_identical_bad_order_files() {
    new_ucmd!()
        .args(&["bad_order_1", "bad_order_1"])
        .succeeds()
        .stdout_only_fixture("bad_order11.defaultcheck_order.expected");
}

#[cfg_attr(not(feature = "test_unimplemented"), ignore)]
#[test]
fn defaultcheck_order_two_different_bad_order_files() {
    new_ucmd!()
        .args(&["bad_order_1", "bad_order_2"])
        .fails()
        .stdout_is_fixture("bad_order12.nocheck_order.expected")
        .stderr_is("error to be defined");
}

// * the third: (it is not know whether this is a bug or not)
// for the first incident, and only the first incident,
// where both lines are different and one or both file lines being
// compared are out of order from the preceding line,
// it is ignored and no errors occur.

// * the fourth: (it is not known whether this is a bug or not)
// there are additional, not-yet-understood circumstances where an out-of-order
// pair is ignored and is not counted against the 1 maximum out-of-order line.

#[cfg_attr(not(feature = "test_unimplemented"), ignore)]
#[test]
fn unintuitive_default_behavior_1() {
    new_ucmd!()
        .args(&["defaultcheck_unintuitive_1", "defaultcheck_unintuitive_2"])
        .succeeds()
        .stdout_only_fixture("defaultcheck_unintuitive.expected");
}

#[ignore] //bug? should help be stdout if not called via -h|--help?
#[test]
fn no_arguments() {
    new_ucmd!().fails().no_stdout().no_stderr();
}

#[ignore] //bug? should help be stdout if not called via -h|--help?
#[test]
fn one_argument() {
    new_ucmd!().arg("a").fails().no_stdout().no_stderr();
}

#[test]
fn test_no_such_file() {
    new_ucmd!()
        .args(&["bogus_file_1", "bogus_file_2"])
        .fails()
        .stderr_only("comm: bogus_file_1: No such file or directory");
}
