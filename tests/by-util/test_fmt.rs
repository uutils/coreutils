// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use uutests::new_ucmd;
use uutests::util::TestScenario;
use uutests::util_name;

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails().code_is(1);
}

#[test]
fn test_invalid_input() {
    new_ucmd!().arg(".").fails().code_is(1);
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
    for param in ["-q", "--quick", "-qq"] {
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
    new_ucmd!()
        .args(&["one-word-per-line.txt", "-w50", "--width", "10"])
        .succeeds()
        .stdout_is("this is a\nfile with\none word\nper line\n");
}

#[test]
fn test_fmt_width_invalid() {
    new_ucmd!()
        .args(&["one-word-per-line.txt", "-w", "apple"])
        .fails()
        .code_is(1)
        .no_stdout()
        .stderr_is("fmt: invalid width: 'apple'\n");
    // an invalid width can be successfully overwritten later:
    new_ucmd!()
        .args(&["one-word-per-line.txt", "-w", "apple", "-w10"])
        .succeeds()
        .stdout_is("this is a\nfile with\none word\nper line\n");
}

#[test]
fn test_fmt_positional_width() {
    new_ucmd!()
        .args(&["-10", "one-word-per-line.txt"])
        .succeeds()
        .stdout_is("this is a\nfile with\none word\nper line\n");
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
    // However, as a temporary value it is okay:
    new_ucmd!()
        .args(&["one-word-per-line.txt", "-w2501", "--width", "10"])
        .succeeds()
        .stdout_is("this is a\nfile with\none word\nper line\n");
}

#[test]
fn test_fmt_invalid_width() {
    for param in ["-w", "--width"] {
        new_ucmd!()
            .args(&["one-word-per-line.txt", param, "invalid"])
            .fails()
            .code_is(1)
            .stderr_contains("invalid width: 'invalid'");
    }
}

#[test]
fn test_fmt_positional_width_not_first() {
    new_ucmd!()
        .args(&["one-word-per-line.txt", "-10"])
        .fails()
        .code_is(1)
        .stderr_contains("fmt: invalid option -- 1; -WIDTH is recognized only when it is the first\noption; use -w N instead");
}

#[test]
fn test_fmt_width_not_valid_number() {
    new_ucmd!()
        .args(&["-25x", "one-word-per-line.txt"])
        .fails()
        .code_is(1)
        .stderr_contains("fmt: invalid width: '25x'");
}

#[ignore = "our 'goal' algorithm is very different from GNU; fix this!"]
#[test]
fn test_fmt_goal() {
    for param in ["-g", "--goal"] {
        new_ucmd!()
            .args(&["one-word-per-line.txt", param, "7"])
            .succeeds()
            .stdout_is("this is a\nfile with one\nword per line\n");
    }
    new_ucmd!()
        .args(&["one-word-per-line.txt", "-g40", "-g7"])
        .succeeds()
        .stdout_is("this is a\nfile with one\nword per line\n");
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

#[ignore = "our 'goal' algorithm is very different from GNU; fix this!"]
#[test]
fn test_fmt_too_big_goal_sometimes_okay() {
    new_ucmd!()
        .args(&["one-word-per-line.txt", "--width=75", "-g76", "-g10"])
        .succeeds()
        .stdout_is("this is a\nfile with one\nword per line\n");
    new_ucmd!()
        .args(&["one-word-per-line.txt", "-g76", "-g10"])
        .succeeds()
        .stdout_is("this is a\nfile with one\nword per line\n");
}

#[test]
fn test_fmt_goal_too_small_to_check_negative_minlength() {
    for param in ["-g", "--goal"] {
        new_ucmd!()
            .args(&["one-word-per-line.txt", "--width=75", param, "10"])
            .succeeds()
            .stdout_is("this is a file with one word per line\n");
    }
}

#[test]
fn test_fmt_non_existent_file() {
    new_ucmd!()
        .args(&["non-existing"])
        .fails()
        .code_is(1)
        .stderr_is("fmt: cannot open 'non-existing' for reading: No such file or directory\n");
}

#[test]
fn test_fmt_invalid_goal() {
    for param in ["-g", "--goal"] {
        new_ucmd!()
            .args(&["one-word-per-line.txt", param, "invalid"])
            .fails()
            .code_is(1)
            // GNU complains about "invalid width", which is confusing.
            // We intentionally deviate from GNU, and show a more helpful message:
            .stderr_contains("invalid goal: 'invalid'");
    }
}

#[test]
fn test_fmt_invalid_goal_override() {
    new_ucmd!()
        .args(&["one-word-per-line.txt", "-g", "apple", "-g", "74"])
        .succeeds()
        .stdout_is("this is a file with one word per line\n");
}

#[test]
fn test_fmt_invalid_goal_width_priority() {
    new_ucmd!()
        .args(&["one-word-per-line.txt", "-g", "apple", "-w", "banana"])
        .fails()
        .code_is(1)
        .no_stdout()
        .stderr_is("fmt: invalid width: 'banana'\n");
    new_ucmd!()
        .args(&["one-word-per-line.txt", "-w", "banana", "-g", "apple"])
        .fails()
        .code_is(1)
        .no_stdout()
        .stderr_is("fmt: invalid width: 'banana'\n");
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

#[test]
fn split_does_not_reflow() {
    for arg in ["-s", "-ss", "--split-only"] {
        new_ucmd!()
            .arg("one-word-per-line.txt")
            .arg(arg)
            .succeeds()
            .stdout_is_fixture("one-word-per-line.txt");
    }
}

#[test]
fn prefix_minus() {
    for prefix_args in [
        vec!["-p-"],
        vec!["-p", "-"],
        vec!["--prefix=-"],
        vec!["--prefix", "-"],
        vec!["--pref=-"],
        vec!["--pref", "-"],
        // Test self-overriding:
        vec!["--prefix==", "--prefix=-"],
    ] {
        new_ucmd!()
            .args(&prefix_args)
            .arg("prefixed-one-word-per-line.txt")
            .succeeds()
            .stdout_is_fixture("prefixed-one-word-per-line_p-.txt");
    }
}

#[test]
fn prefix_equal() {
    for prefix_args in [
        // FIXME: #6353 vec!["-p="],
        vec!["-p", "="],
        vec!["--prefix=="],
        vec!["--prefix", "="],
        vec!["--pref=="],
        vec!["--pref", "="],
        // Test self-overriding:
        vec!["--prefix=-", "--prefix=="],
    ] {
        new_ucmd!()
            .args(&prefix_args)
            .arg("prefixed-one-word-per-line.txt")
            .succeeds()
            .stdout_is_fixture("prefixed-one-word-per-line_p=.txt");
    }
}

#[test]
fn prefix_equal_skip_prefix_equal_two() {
    for prefix_args in [
        // FIXME: #6353 vec!["--prefix==", "-P=2"],
        vec!["--prefix==", "-P", "=2"],
        vec!["--prefix==", "--skip-prefix==2"],
        vec!["--prefix==", "--skip-prefix", "=2"],
        vec!["--prefix==", "--skip-pref==2"],
        vec!["--prefix==", "--skip-pref", "=2"],
        // Test self-overriding:
        vec!["--prefix==", "--skip-pref", "asdf", "-P", "=2"],
    ] {
        new_ucmd!()
            .args(&prefix_args)
            .arg("prefixed-one-word-per-line.txt")
            .succeeds()
            .stdout_is_fixture("prefixed-one-word-per-line_p=_P=2.txt");
    }
}
