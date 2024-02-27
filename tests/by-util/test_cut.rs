// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use crate::common::util::TestScenario;

static INPUT: &str = "lists.txt";

struct TestedSequence<'b> {
    name: &'b str,
    sequence: &'b str,
}

static EXAMPLE_SEQUENCES: &[TestedSequence] = &[
    TestedSequence {
        name: "singular",
        sequence: "2",
    },
    TestedSequence {
        name: "prefix",
        sequence: "-2",
    },
    TestedSequence {
        name: "suffix",
        sequence: "2-",
    },
    TestedSequence {
        name: "range",
        sequence: "2-4",
    },
    TestedSequence {
        name: "aggregate",
        sequence: "9-,6-7,-2,4",
    },
    TestedSequence {
        name: "subsumed",
        sequence: "2-,3",
    },
];

static COMPLEX_SEQUENCE: &TestedSequence = &TestedSequence {
    name: "",
    sequence: "9-,6-7,-2,4",
};

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails().code_is(1);
}

#[test]
fn test_byte_sequence() {
    for param in ["-b", "--bytes", "--byt"] {
        for example_seq in EXAMPLE_SEQUENCES {
            new_ucmd!()
                .args(&[param, example_seq.sequence, INPUT])
                .succeeds()
                .stdout_only_fixture(format!("sequences/byte_{}.expected", example_seq.name));
        }
    }
}

#[test]
fn test_char_sequence() {
    for param in ["-c", "--characters", "--char"] {
        for example_seq in EXAMPLE_SEQUENCES {
            //as of coreutils 8.25 a char range is effectively the same as a byte range; there is no distinct treatment of utf8 chars.
            new_ucmd!()
                .args(&[param, example_seq.sequence, INPUT])
                .succeeds()
                .stdout_only_fixture(format!("sequences/byte_{}.expected", example_seq.name));
        }
    }
}

#[test]
fn test_field_sequence() {
    for param in ["-f", "--fields", "--fie"] {
        for example_seq in EXAMPLE_SEQUENCES {
            new_ucmd!()
                .args(&[param, example_seq.sequence, INPUT])
                .succeeds()
                .stdout_only_fixture(format!("sequences/field_{}.expected", example_seq.name));
        }
    }
}

#[test]
fn test_whitespace_delimited() {
    new_ucmd!()
        .args(&["-w", "-f", COMPLEX_SEQUENCE.sequence, INPUT])
        .succeeds()
        .stdout_only_fixture("whitespace_delimited.expected");
}

#[test]
fn test_whitespace_with_explicit_delimiter() {
    new_ucmd!()
        .args(&["-w", "-f", COMPLEX_SEQUENCE.sequence, "-d:"])
        .fails()
        .code_is(1);
}

#[test]
fn test_whitespace_with_byte() {
    new_ucmd!()
        .args(&["-w", "-b", COMPLEX_SEQUENCE.sequence])
        .fails()
        .code_is(1);
}

#[test]
fn test_whitespace_with_char() {
    new_ucmd!()
        .args(&["-c", COMPLEX_SEQUENCE.sequence, "-w"])
        .fails()
        .code_is(1);
}

#[test]
fn test_too_large() {
    new_ucmd!()
        .args(&["-b1-18446744073709551615", "/dev/null"])
        .fails()
        .code_is(1);
}

#[test]
fn test_delimiter() {
    for param in ["-d", "--delimiter", "--del"] {
        new_ucmd!()
            .args(&[param, ":", "-f", COMPLEX_SEQUENCE.sequence, INPUT])
            .succeeds()
            .stdout_only_fixture("delimiter_specified.expected");
    }
}

#[test]
fn test_delimiter_with_more_than_one_char() {
    new_ucmd!()
        .args(&["-d", "ab", "-f1"])
        .fails()
        .stderr_contains("cut: the delimiter must be a single character")
        .no_stdout();
}

#[test]
fn test_output_delimiter() {
    // we use -d here to ensure output delimiter
    // is applied to the current, and not just the default, input delimiter
    new_ucmd!()
        .args(&[
            "-d:",
            "--output-delimiter=@",
            "-f",
            COMPLEX_SEQUENCE.sequence,
            INPUT,
        ])
        .succeeds()
        .stdout_only_fixture("output_delimiter.expected");

    new_ucmd!()
        .args(&[
            "-d:",
            "--output-del=@",
            "-f",
            COMPLEX_SEQUENCE.sequence,
            INPUT,
        ])
        .succeeds()
        .stdout_only_fixture("output_delimiter.expected");
}

#[test]
fn test_complement() {
    for param in ["--complement", "--com"] {
        new_ucmd!()
            .args(&["-d_", param, "-f", "2"])
            .pipe_in("9_1\n8_2\n7_3")
            .succeeds()
            .stdout_only("9\n8\n7\n");
    }
}

#[test]
fn test_zero_terminated() {
    new_ucmd!()
        .args(&["-d_", "-z", "-f", "1"])
        .pipe_in("9_1\n8_2\n\x007_3")
        .succeeds()
        .stdout_only("9\x007\0");
}

#[test]
fn test_only_delimited() {
    for param in ["-s", "--only-delimited", "--only-del"] {
        new_ucmd!()
            .args(&["-d_", param, "-f", "1"])
            .pipe_in("91\n82\n7_3")
            .succeeds()
            .stdout_only("7\n");
    }
}

#[test]
fn test_zero_terminated_only_delimited() {
    new_ucmd!()
        .args(&["-d_", "-z", "-s", "-f", "1"])
        .pipe_in("91\n\082\n7_3")
        .succeeds()
        .stdout_only("82\n7\0");
}

#[test]
fn test_is_a_directory() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.mkdir("some");

    ucmd.arg("-b1")
        .arg("some")
        .fails()
        .code_is(1)
        .stderr_is("cut: some: Is a directory\n");
}

#[test]
fn test_no_such_file() {
    new_ucmd!()
        .arg("-b1")
        .arg("some")
        .fails()
        .code_is(1)
        .stderr_is("cut: some: No such file or directory\n");
}

#[test]
fn test_equal_as_delimiter1() {
    new_ucmd!()
        .args(&["-f", "2", "-d="])
        .pipe_in("--dir=./out/lib")
        .succeeds()
        .stdout_only("./out/lib\n");
}

#[test]
fn test_equal_as_delimiter2() {
    new_ucmd!()
        .args(&["-f2", "--delimiter="])
        .pipe_in("a=b\n")
        .succeeds()
        .stdout_only("a=b\n");
}

#[test]
fn test_equal_as_delimiter3() {
    new_ucmd!()
        .args(&["-f", "1,2", "-d", "''", "--output-delimiter=Z"])
        .pipe_in("ab\0cd\n")
        .succeeds()
        .stdout_only_bytes("abZcd\n");
}

#[test]
fn test_multiple() {
    let result = new_ucmd!()
        .args(&["-f2", "-d:", "-d="])
        .pipe_in("a=b\n")
        .succeeds();
    assert_eq!(result.stdout_str(), "b\n");
    assert_eq!(result.stderr_str(), "");
}

#[test]
fn test_multiple_mode_args() {
    for args in [
        vec!["-b1", "-b2"],
        vec!["-c1", "-c2"],
        vec!["-f1", "-f2"],
        vec!["-b1", "-c2"],
        vec!["-b1", "-f2"],
        vec!["-c1", "-f2"],
        vec!["-b1", "-c2", "-f3"],
    ] {
        new_ucmd!()
        .args(&args)
        .fails()
        .stderr_is("cut: invalid usage: expects no more than one of --fields (-f), --chars (-c) or --bytes (-b)\n");
    }
}
