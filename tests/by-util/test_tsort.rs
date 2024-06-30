// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
#![allow(clippy::cast_possible_wrap)]

use crate::common::util::TestScenario;

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails().code_is(1);
}
#[test]
fn test_sort_call_graph() {
    new_ucmd!()
        .arg("call_graph.txt")
        .run()
        .stdout_is_fixture("call_graph.expected");
}

#[test]
fn test_sort_self_loop() {
    new_ucmd!()
        .pipe_in("first first\nfirst second second second")
        .succeeds()
        .stdout_only("first\nsecond\n");
}

#[test]
fn test_sort_floating_nodes() {
    new_ucmd!()
        .pipe_in("d d\nc c\na a\nb b")
        .succeeds()
        .stdout_only("a\nb\nc\nd\n");
}

#[test]
fn test_no_such_file() {
    new_ucmd!()
        .arg("invalid_file_txt")
        .fails()
        .stderr_contains("No such file or directory");
}

#[test]
fn test_version_flag() {
    let version_short = new_ucmd!().arg("-V").succeeds();
    let version_long = new_ucmd!().arg("--version").succeeds();

    assert_eq!(version_short.stdout_str(), version_long.stdout_str());
}

#[test]
fn test_help_flag() {
    let help_short = new_ucmd!().arg("-h").succeeds();
    let help_long = new_ucmd!().arg("--help").succeeds();

    assert_eq!(help_short.stdout_str(), help_long.stdout_str());
}

#[test]
fn test_multiple_arguments() {
    new_ucmd!()
        .arg("call_graph.txt")
        .arg("invalid_file")
        .fails()
        .stderr_contains("unexpected argument 'invalid_file' found");
}

#[test]
fn test_error_on_dir() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir("tsort_test_dir");
    ucmd.arg("tsort_test_dir")
        .fails()
        .stderr_contains("tsort: tsort_test_dir: read error: Is a directory");
}
