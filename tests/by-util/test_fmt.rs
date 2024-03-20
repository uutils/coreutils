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
fn test_small_width() {
    for width in ["0", "1", "2", "3"] {
        for param in ["-w", "--width"] {
            new_ucmd!()
                .args(&[param, width, "one-word-per-line.txt"])
                .succeeds()
                .stdout_is("this\nis\na\nfile\nwith\none\nword\nper\nline\n");
        }
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
fn test_fmt_invalid_width() {
    for param in ["-w", "--width"] {
        new_ucmd!()
            .args(&["one-word-per-line.txt", param, "invalid"])
            .fails()
            .code_is(1)
            .stderr_contains("invalid value 'invalid'");
    }
}

#[ignore]
#[test]
fn test_fmt_goal() {
    for param in ["-g", "--goal"] {
        new_ucmd!()
            .args(&["one-word-per-line.txt", param, "7"])
            .succeeds()
            .stdout_is("this is a\nfile with one\nword per line\n");
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
fn test_fmt_goal_bigger_than_default_width_of_75() {
    for param in ["-g", "--goal"] {
        new_ucmd!()
            .args(&["one-word-per-line.txt", param, "76"])
            .fails()
            .code_is(1)
            .stderr_is("fmt: GOAL cannot be greater than WIDTH.\n");
    }
}

#[test]
fn test_fmt_invalid_goal() {
    for param in ["-g", "--goal"] {
        new_ucmd!()
            .args(&["one-word-per-line.txt", param, "invalid"])
            .fails()
            .code_is(1)
            .stderr_contains("invalid value 'invalid'");
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
