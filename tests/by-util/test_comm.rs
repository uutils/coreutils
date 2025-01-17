// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore (words) defaultcheck nocheck helpb helpz nwordb nwordwordz wordtotal

use crate::common::util::TestScenario;

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails().code_is(1);
}

#[test]
fn ab_no_args() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.write("a", "a\nz\n");
    at.write("b", "b\nz\n");

    scene
        .ucmd()
        .args(&["a", "b"])
        .succeeds()
        .stdout_is("a\n\tb\n\t\tz\n");
}

#[test]
fn ab_dash_one() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.write("a", "a\nz\n");
    at.write("b", "b\nz\n");

    scene
        .ucmd()
        .args(&["a", "b", "-1"])
        .succeeds()
        .stdout_is("b\n\tz\n");
}

#[test]
fn ab_dash_two() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.write("a", "a\nz\n");
    at.write("b", "b\nz\n");

    scene
        .ucmd()
        .args(&["a", "b", "-2"])
        .succeeds()
        .stdout_is("a\n\tz\n");
}

#[test]
fn ab_dash_three() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.write("a", "a\nz\n");
    at.write("b", "b\nz\n");

    scene
        .ucmd()
        .args(&["a", "b", "-3"])
        .succeeds()
        .stdout_is("a\n\tb\n");
}

#[test]
fn a_empty() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.write("a", "a\nz\n");
    at.touch("empty");
    scene
        .ucmd()
        .args(&["a", "empty"])
        .succeeds()
        .stdout_is("a\nz\n");
}

#[test]
fn empty_empty() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.touch("empty");
    scene
        .ucmd()
        .args(&["empty", "empty"])
        .succeeds()
        .no_output();
}

#[test]
fn total() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.write("a", "a\nz\n");
    at.write("b", "b\nz\n");
    scene
        .ucmd()
        .args(&["--total", "a", "b"])
        .succeeds()
        .stdout_is("a\n\tb\n\t\tz\n1\t1\t1\ttotal\n");
}

#[test]
fn total_with_suppressed_regular_output() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.write("a", "a\nz\n");
    at.write("b", "b\nz\n");
    scene
        .ucmd()
        .args(&["--total", "-123", "a", "b"])
        .succeeds()
        .stdout_is("1\t1\t1\ttotal\n");
}

#[test]
fn repeated_flags() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.write("a", "a\nz\n");
    at.write("b", "b\nz\n");
    scene
        .ucmd()
        .args(&["--total", "-123123", "--total", "a", "b"])
        .succeeds()
        .stdout_is("1\t1\t1\ttotal\n");
}

#[test]
fn total_with_output_delimiter() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.write("a", "a\nz\n");
    at.write("b", "b\nz\n");
    scene
        .ucmd()
        .args(&["--total", "--output-delimiter=word", "a", "b"])
        .succeeds()
        .stdout_is("a\nwordb\nwordwordz\n1word1word1wordtotal\n");
}

#[test]
fn output_delimiter() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.write("a", "a\nz\n");
    at.write("b", "b\nz\n");
    scene
        .ucmd()
        .args(&["--output-delimiter=word", "a", "b"])
        .succeeds()
        .stdout_is("a\nwordb\nwordwordz\n");
}

#[test]
fn output_delimiter_hyphen_one() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.write("a", "a\nz\n");
    at.write("b", "b\nz\n");
    scene
        .ucmd()
        .args(&["--output-delimiter", "-1", "a", "b"])
        .succeeds()
        .stdout_is("a\n-1b\n-1-1z\n");
}

#[test]
fn output_delimiter_hyphen_help() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.write("a", "a\nz\n");
    at.write("b", "b\nz\n");
    scene
        .ucmd()
        .args(&["--output-delimiter", "--help", "a", "b"])
        .succeeds()
        .stdout_is("a\n--helpb\n--help--helpz\n");
}

#[test]
fn output_delimiter_multiple_identical() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.write("a", "a\nz\n");
    at.write("b", "b\nz\n");
    scene
        .ucmd()
        .args(&[
            "--output-delimiter=word",
            "--output-delimiter=word",
            "a",
            "b",
        ])
        .succeeds()
        .stdout_is("a\nwordb\nwordwordz\n");
}

#[test]
fn output_delimiter_multiple_different() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.write("a", "a\nz\n");
    at.write("b", "b\nz\n");
    scene
        .ucmd()
        .args(&[
            "--output-delimiter=word",
            "--output-delimiter=other",
            "a",
            "b",
        ])
        .fails()
        .no_stdout()
        .stderr_contains("multiple")
        .stderr_contains("output")
        .stderr_contains("delimiters");
}

#[test]
#[ignore = "This is too weird; deviate intentionally."]
fn output_delimiter_multiple_different_prevents_help() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.write("a", "a\nz\n");
    at.write("b", "b\nz\n");
    scene
        .ucmd()
        .args(&[
            "--output-delimiter=word",
            "--output-delimiter=other",
            "--help",
            "a",
            "b",
        ])
        .fails()
        .no_stdout()
        .stderr_contains("multiple")
        .stderr_contains("output")
        .stderr_contains("delimiters");
}

#[test]
fn output_delimiter_nul() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.write("a", "a\nz\n");
    at.write("b", "b\nz\n");
    scene
        .ucmd()
        .args(&["--output-delimiter=", "a", "b"])
        .succeeds()
        .stdout_is("a\n\0b\n\0\0z\n");
}

#[test]
fn zero_terminated() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.write("a_nul", "a\0z\0");
    at.write("b_nul", "b\0z\0");
    for param in ["-z", "--zero-terminated"] {
        scene
            .ucmd()
            .args(&[param, "a_nul", "b_nul"])
            .succeeds()
            .stdout_is("a\0\tb\0\t\tz\0");
    }
}

#[test]
fn zero_terminated_provided_multiple_times() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.write("a_nul", "a\0z\0");
    at.write("b_nul", "b\0z\0");
    for param in ["-z", "--zero-terminated"] {
        scene
            .ucmd()
            .args(&[param, param, param, "a_nul", "b_nul"])
            .succeeds()
            .stdout_is("a\0\tb\0\t\tz\0");
    }
}

#[test]
fn zero_terminated_with_total() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.write("a_nul", "a\0z\0");
    at.write("b_nul", "b\0z\0");

    for param in ["-z", "--zero-terminated"] {
        scene
            .ucmd()
            .args(&[param, "--total", "a_nul", "b_nul"])
            .succeeds()
            .stdout_is("a\0\tb\0\t\tz\x001\t1\t1\ttotal\0");
    }
}

#[cfg_attr(not(feature = "test_unimplemented"), ignore)]
#[test]
fn check_order() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.write("bad_order_1", "e\nd\nb\na\n");
    at.write("bad_order_2", "e\nc\nb\na\n");
    scene
        .ucmd()
        .args(&["--check-order", "bad_order_1", "bad_order_2"])
        .fails()
        .stdout_is("\t\te")
        .stderr_is("error to be defined");
}

#[cfg_attr(not(feature = "test_unimplemented"), ignore)]
#[test]
fn nocheck_order() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.write("bad_order_1", "e\nd\nb\na\n");
    at.write("bad_order_2", "e\nc\nb\na\n");
    new_ucmd!()
        .args(&["--nocheck-order", "bad_order_1", "bad_order_2"])
        .succeeds()
        .stdout_is("\t\te\n\tc\n\tb\n\ta\nd\nb\na\n");
}

// when neither --check-order nor --no-check-order is provided,
// stderr and the error code behaves like check order, but stdout
// behaves like nocheck_order. However with some quirks detailed below.
#[cfg_attr(not(feature = "test_unimplemented"), ignore)]
#[test]
fn defaultcheck_order() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.write("bad_order_1", "e\nd\nb\na\n");
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
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.write("bad_order_1", "e\nd\nb\na\n");
    scene
        .ucmd()
        .args(&["bad_order_1", "bad_order_1"])
        .succeeds()
        .stdout_is("\t\te\n\t\td\n\t\tb\n\t\ta\n");
}

#[cfg_attr(not(feature = "test_unimplemented"), ignore)]
#[test]
fn defaultcheck_order_two_different_bad_order_files() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.write("bad_order_1", "e\nd\nb\na\n");
    at.write("bad_order_2", "e\nc\nb\na\n");
    scene
        .ucmd()
        .args(&["bad_order_1", "bad_order_2"])
        .fails()
        .stdout_is("\t\te\n\tc\n\tb\n\ta\nd\nb\na\n")
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
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.write("defaultcheck_unintuitive_1", "m\nh\nn\no\nc\np\n");
    at.write("defaultcheck_unintuitive_2", "m\nh\nn\no\np\n");
    scene
        .ucmd()
        .args(&["defaultcheck_unintuitive_1", "defaultcheck_unintuitive_2"])
        .succeeds()
        .stdout_is("\t\tm\n\t\th\n\t\tn\n\t\to\nc\n\t\tp\n");
}

#[test]
fn no_arguments() {
    new_ucmd!().fails().no_stdout();
}

#[test]
fn one_argument() {
    new_ucmd!().arg("a").fails().no_stdout();
}

#[test]
fn test_no_such_file() {
    new_ucmd!()
        .args(&["bogus_file_1", "bogus_file_2"])
        .fails()
        .stderr_only("comm: bogus_file_1: No such file or directory\n");
}

#[test]
fn test_is_dir() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    scene
        .ucmd()
        .args(&[".", "."])
        .fails()
        .stderr_only("comm: .: Is a directory\n");

    at.mkdir("dir");
    scene
        .ucmd()
        .args(&["dir", "."])
        .fails()
        .stderr_only("comm: dir: Is a directory\n");

    at.touch("file");
    scene
        .ucmd()
        .args(&[".", "file"])
        .fails()
        .stderr_only("comm: .: Is a directory\n");

    at.touch("file");
    scene
        .ucmd()
        .args(&["file", "."])
        .fails()
        .stderr_only("comm: .: Is a directory\n");
}
