use common::util::*;
use std::ffi::OsStr;

static UTIL_NAME: &'static str = "comm";

fn at_and_ucmd() -> (AtPath, UCommand) {
    let ts = TestScenario::new(UTIL_NAME);
    let ucmd = ts.ucmd();
    (ts.fixtures, ucmd)
}

fn new_ucmd() -> UCommand {
    TestScenario::new(UTIL_NAME).ucmd()
}

fn comm<A: AsRef<OsStr>, B: AsRef<str>>(args: &[A],
                                                       file_stdout_relpath_opt: Option<B>,
                                                       error_message_opt: Option<B>) {
    let (at, mut ucmd) = at_and_ucmd();
    let result = ucmd.args(args)
                     .run();
    assert!(result.success == error_message_opt.is_none());
    if let Some(_) = error_message_opt {
        assert!(result.stderr.len() > 0);
        // assert!(result.stderr.trim_right() == s);
    } else {
        assert!(result.stderr.len() == 0);
    }
    if let Some(file_stdout_relpath) = file_stdout_relpath_opt {
        assert!(result.stdout == at.read(file_stdout_relpath.as_ref()))
    }
}

#[test]
fn ab_no_args() {
    comm(&["a", "b"], Some("ab.expected"), None);
}

#[test]
fn ab_dash_one() {
    comm(&["a", "b", "-1"], Some("ab1.expected"), None);
}

#[test]
fn ab_dash_two() {
    comm(&["a", "b", "-2"], Some("ab2.expected"), None);
}

#[test]
fn ab_dash_three() {
    comm(&["a", "b", "-3"], Some("ab3.expected"), None);
}

#[test]
fn aempty() {
    comm(&["a", "empty"], Some("aempty.expected"), None);
}

#[test]
fn emptyempty() {
    comm(&["empty", "empty"], Some("emptyempty.expected"), None);
}

#[cfg_attr(not(feature="test_unimplemented"),ignore)]
#[test]
fn output_delimiter() {
    comm(&["--output-delimiter=word", "a", "b"],
               Some("ab_delimiter_word.expected"),
               None);
}

#[cfg_attr(not(feature="test_unimplemented"),ignore)]
#[test]
fn output_delimiter_require_arg() {
    comm(&["--output-delimiter=", "a", "b"],
               None,
               Some("error to be defined"));
}

// even though (info) documentation suggests this is an option
// in latest GNU Coreutils comm, it actually is not.
// this test is essentially an alarm in case someone well-intendingly
// implements it.
#[test]
fn zero_terminated() {
    for param in vec!["-z", "--zero-terminated"] {
        comm(&[param, "a", "b"], None, Some("error to be defined"));
    }
}

#[cfg_attr(not(feature="test_unimplemented"),ignore)]
#[test]
fn check_order() {
    comm(&["--check-order", "bad_order_1", "bad_order_2"],
               Some("bad_order12.check_order.expected"),
               Some("error to be defined"));
}

#[cfg_attr(not(feature="test_unimplemented"),ignore)]
#[test]
fn nocheck_order() {
    comm(&["--nocheck-order", "bad_order_1", "bad_order_2"],
               Some("bad_order12.nocheck_order.expected"),
               None);
}

// when neither --check-order nor --no-check-order is provided,
// stderr and the error code behaves like check order, but stdout
// behaves like nocheck_order. However with some quirks detailed below.
#[cfg_attr(not(feature="test_unimplemented"),ignore)]
#[test]
fn defaultcheck_order() {
    comm(&["a", "bad_order_1"], None, Some("error to be defined"));
}

// * the first: if both files are not in order, the default behavior is the only
// behavior that will provide an error message

// * the second: if two rows are paired but are out of order,
// it won't matter if all rows in the two files are exactly the same.
// This is specified in the documentation

#[test]
fn defaultcheck_order_identical_bad_order_files() {
    comm(&["bad_order_1", "bad_order_1"],
               Some("bad_order11.defaultcheck_order.expected"),
               None);
}

#[cfg_attr(not(feature="test_unimplemented"),ignore)]
#[test]
fn defaultcheck_order_two_different_bad_order_files() {
    comm(&["bad_order_1", "bad_order_2"],
               Some("bad_order12.nocheck_order.expected"),
               Some("error to be defined"));
}

// * the third: (it is not know whether this is a bug or not)
// for the first incident, and only the first incident,
// where both lines are different and one or both file lines being
// compared are out of order from the preceding line,
// it is ignored and no errors occur.

// * the fourth: (it is not known whether this is a bug or not)
// there are additional, not-yet-understood circumstances where an out-of-order
// pair is ignored and is not counted against the 1 maximum out-of-order line.

#[cfg_attr(not(feature="test_unimplemented"),ignore)]
#[test]
fn unintuitive_default_behavior_1() {
    comm(&["defaultcheck_unintuitive_1", "defaultcheck_unintuitive_2"],
               Some("defaultcheck_unintuitive.expected"),
               None);
}

#[ignore] //bug? should help be stdout if not called via -h|--help?
#[test]
fn no_arguments() {
    let result = new_ucmd()
        .run();
    assert!(!result.success);
    assert!(result.stdout.len() == 0);
    assert!(result.stderr.len() > 0);
}

#[ignore] //bug? should help be stdout if not called via -h|--help?
#[test]
fn one_argument() {
    let result = new_ucmd()
        .arg("a").run();
    assert!(!result.success);
    assert!(result.stdout.len() == 0);
    assert!(result.stderr.len() > 0);
}
