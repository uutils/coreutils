// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use crate::common::util::TestScenario;

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails().code_is(1);
}

#[test]
fn test_fmt() {
    new_ucmd!()
        .arg("one-word-per-line.txt")
        .succeeds()
        .stdout_is("this is a file with one word per line\n");
}

#[test]
fn test_fmt_quick() {
    for param in ["-q", "--quick"] {
        new_ucmd!()
            .args(&["one-word-per-line.txt", param])
            .succeeds()
            .stdout_is("this is a file with one word per line\n");
    }
}

#[test]
fn test_fmt_width() {
    for param in ["-w", "--width"] {
        new_ucmd!()
            .args(&["one-word-per-line.txt", param, "10"])
            .succeeds()
            .stdout_is("this is a\nfile with\none word\nper line\n");
    }
}

#[test]
fn test_fmt_width_too_big() {
    for param in ["-w", "--width"] {
        new_ucmd!()
            .args(&["one-word-per-line.txt", param, "2501"])
            .fails()
            .code_is(1)
            .stderr_is("fmt: invalid width: '2501': Numerical result out of range\n");
    }
}

#[test]
fn test_fmt_goal_input_7() {
    for param in ["-g", "--goal"] {
        new_ucmd!()
            .args(&["one-word-per-line.txt", param, "7"])
            .succeeds()
            .stdout_is("this is a\nfile with one\nword per line\n");
    }
}

#[test]
fn test_fmt_goal_input_6() {
    for param in ["-g", "--goal"] {
        new_ucmd!()
            .args(&["one-word-per-line.txt", param, "6"])
            .succeeds()
            .stdout_is("this is\na file\nwith one\nword per line\n");
    }
}

#[test]
fn test_fmt_goal_input_8() {
    for param in ["-g", "--goal"] {
        new_ucmd!()
            .args(&["one-word-per-line.txt", param, "8"])
            .succeeds()
            .stdout_is("this is a\nfile with\none word per line\n");
    }
}

#[test]
fn test_fmt_goal_input_9() {
    for param in ["-g", "--goal"] {
        new_ucmd!()
            .args(&["one-word-per-line.txt", param, "9"])
            .succeeds()
            .stdout_is("this is a\nfile with\none word per line\n");
    }
}

#[test]
fn test_fmt_goal_input_5() {
    for param in ["-g", "--goal"] {
        new_ucmd!()
            .args(&["one-word-per-line.txt", param, "5"])
            .succeeds()
            .stdout_is("this is\na file\nwith one\nword per line\n");
    }
}

#[test]
fn test_fmt_goal_input_4() {
    for param in ["-g", "--goal"] {
        new_ucmd!()
            .args(&["one-word-per-line.txt", param, "4"])
            .succeeds()
            .stdout_is("this is\na file\nwith one\nword per line\n");
    }
}

#[test]
fn test_fmt_goal_input_3() {
    for param in ["-g", "--goal"] {
        new_ucmd!()
            .args(&["one-word-per-line.txt", param, "3"])
            .succeeds()
            .stdout_is("this is a\nfile with\none word\nper line\n");
    }
}

#[test]
fn test_fmt_goal_input_2() {
    for param in ["-g", "--goal"] {
        new_ucmd!()
            .args(&["one-word-per-line.txt", param, "2"])
            .succeeds()
            .stdout_is("this is a\nfile with\none word\nper line\n");
    }
}

#[test]
fn test_fmt_goal_too_big() {
    for param in ["-g", "--goal"] {
        new_ucmd!()
            .args(&["one-word-per-line.txt", "--width=75", param, "76"])
            .fails()
            .code_is(1)
            .stderr_is("fmt: GOAL cannot be greater than WIDTH.\n");
    }
}

#[test]
fn test_fmt_set_goal_not_contain_width() {
    for param in ["-g", "--goal"] {
        new_ucmd!()
            .args(&["one-word-per-line.txt", param, "74"])
            .succeeds()
            .stdout_is("this is a file with one word per line\n");
    }
}
