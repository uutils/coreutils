// spell-checker:ignore nabcd
use crate::common::util::*;

static INPUT: &str = "sorted.txt";
static OUTPUT: &str = "sorted-output.txt";
static SKIP_CHARS: &str = "skip-chars.txt";
static SKIP_FIELDS: &str = "skip-fields.txt";
static SORTED_ZERO_TERMINATED: &str = "sorted-zero-terminated.txt";

#[test]
fn test_stdin_default() {
    new_ucmd!()
        .pipe_in_fixture(INPUT)
        .run()
        .stdout_is_fixture("sorted-simple.expected");
}

#[test]
fn test_single_default() {
    new_ucmd!()
        .arg(INPUT)
        .run()
        .stdout_is_fixture("sorted-simple.expected");
}

#[test]
fn test_single_default_output() {
    let (at, mut ucmd) = at_and_ucmd!();
    let expected = at.read("sorted-simple.expected");
    ucmd.args(&[INPUT, OUTPUT]).run();
    let found = at.read(OUTPUT);
    assert_eq!(found, expected);
}

#[test]
fn test_stdin_counts() {
    new_ucmd!()
        .args(&["-c"])
        .pipe_in_fixture(INPUT)
        .run()
        .stdout_is_fixture("sorted-counts.expected");
}

#[test]
fn test_stdin_skip_1_char() {
    new_ucmd!()
        .args(&["-s1"])
        .pipe_in_fixture(SKIP_CHARS)
        .run()
        .stdout_is_fixture("skip-1-char.expected");
}

#[test]
fn test_stdin_skip_5_chars() {
    new_ucmd!()
        .args(&["-s5"])
        .pipe_in_fixture(SKIP_CHARS)
        .run()
        .stdout_is_fixture("skip-5-chars.expected");
}

#[test]
fn test_stdin_skip_and_check_2_chars() {
    new_ucmd!()
        .args(&["-s3", "-w2"])
        .pipe_in_fixture(SKIP_CHARS)
        .run()
        .stdout_is_fixture("skip-3-check-2-chars.expected");
}

#[test]
fn test_stdin_skip_1_field() {
    new_ucmd!()
        .args(&["-f2"])
        .pipe_in_fixture(SKIP_FIELDS)
        .run()
        .stdout_is_fixture("skip-2-fields.expected");
}

#[test]
fn test_stdin_all_repeated() {
    new_ucmd!()
        .args(&["--all-repeated"])
        .pipe_in_fixture(INPUT)
        .run()
        .stdout_is_fixture("sorted-all-repeated.expected");
}

#[test]
fn test_stdin_all_repeated_separate() {
    new_ucmd!()
        .args(&["--all-repeated=separate"])
        .pipe_in_fixture(INPUT)
        .run()
        .stdout_is_fixture("sorted-all-repeated-separate.expected");
}

#[test]
fn test_stdin_all_repeated_prepend() {
    new_ucmd!()
        .args(&["--all-repeated=prepend"])
        .pipe_in_fixture(INPUT)
        .run()
        .stdout_is_fixture("sorted-all-repeated-prepend.expected");
}

#[test]
fn test_stdin_unique_only() {
    new_ucmd!()
        .args(&["-u"])
        .pipe_in_fixture(INPUT)
        .run()
        .stdout_is_fixture("sorted-unique-only.expected");
}

#[test]
fn test_stdin_repeated_only() {
    new_ucmd!()
        .args(&["-d"])
        .pipe_in_fixture(INPUT)
        .run()
        .stdout_is_fixture("sorted-repeated-only.expected");
}

#[test]
fn test_stdin_ignore_case() {
    new_ucmd!()
        .args(&["-i"])
        .pipe_in_fixture(INPUT)
        .run()
        .stdout_is_fixture("sorted-ignore-case.expected");
}

#[test]
fn test_stdin_zero_terminated() {
    new_ucmd!()
        .args(&["-z"])
        .pipe_in_fixture(SORTED_ZERO_TERMINATED)
        .run()
        .stdout_is_fixture("sorted-zero-terminated.expected");
}

#[test]
fn test_invalid_utf8() {
    new_ucmd!()
        .arg("not-utf8-sequence.txt")
        .run()
        .failure()
        .stderr_only(
            "uniq: failed to convert line to utf8: invalid utf-8 sequence of 1 bytes from index 0",
        );
}

#[test]
fn test_group() {
    new_ucmd!()
        .args(&["--group"])
        .pipe_in_fixture(INPUT)
        .run()
        .stdout_is_fixture("group.expected");
}

#[test]
fn test_group_prepend() {
    new_ucmd!()
        .args(&["--group=prepend"])
        .pipe_in_fixture(INPUT)
        .run()
        .stdout_is_fixture("group-prepend.expected");
}

#[test]
fn test_group_append() {
    new_ucmd!()
        .args(&["--group=append"])
        .pipe_in_fixture(INPUT)
        .run()
        .stdout_is_fixture("group-append.expected");
}

#[test]
fn test_group_both() {
    new_ucmd!()
        .args(&["--group=both"])
        .pipe_in_fixture(INPUT)
        .run()
        .stdout_is_fixture("group-both.expected");
}

#[test]
fn test_group_separate() {
    new_ucmd!()
        .args(&["--group=separate"])
        .pipe_in_fixture(INPUT)
        .run()
        .stdout_is_fixture("group.expected");
}

#[test]
fn test_case2() {
    new_ucmd!().pipe_in("a\na\n").run().stdout_is("a\n");
}

struct TestCase {
    name: &'static str,
    args: &'static [&'static str],
    input: &'static str,
    stdout: Option<&'static str>,
    stderr: Option<&'static str>,
    exit: Option<i32>,
}

#[test]
fn gnu_tests() {
    let cases = [
        TestCase {
            name: "1",
            args: &[],
            input: "",
            stdout: Some(""),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "2",
            args: &[],
            input: "a\na\n",
            stdout: Some("a\n"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "3",
            args: &[],
            input: "a\na",
            stdout: Some("a\n"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "4",
            args: &[],
            input: "a\nb",
            stdout: Some("a\nb\n"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "5",
            args: &[],
            input: "a\na\nb",
            stdout: Some("a\nb\n"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "6",
            args: &[],
            input: "b\na\na\n",
            stdout: Some("b\na\n"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "7",
            args: &[],
            input: "a\nb\nc\n",
            stdout: Some("a\nb\nc\n"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "2z",
            args: &["-z"],
            input: "a\na\n",
            stdout: Some("a\na\n\0"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "3z",
            args: &["-z"],
            input: "a\na",
            stdout: Some("a\na\0"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "4z",
            args: &["-z"],
            input: "a\nb",
            stdout: Some("a\nb\0"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "5z",
            args: &["-z"],
            input: "a\na\nb",
            stdout: Some("a\na\nb\0"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "10z",
            args: &["-z", "-f1"],
            input: "a\nb\n\0c\nb\n\0",
            stdout: Some("a\nb\n\0"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "20z",
            args: &["-dz"],
            input: "a\na\n",
            stdout: Some(""),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "8",
            args: &[],
            input: "ö\nv\n",
            stdout: Some("ö\nv\n"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "9",
            args: &["-u"],
            input: "a\na\n",
            stdout: Some(""),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "10",
            args: &["-u"],
            input: "a\nb\n",
            stdout: Some("a\nb\n"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "11",
            args: &["-u"],
            input: "a\nb\na\n",
            stdout: Some("a\nb\na\n"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "12",
            args: &["-u"],
            input: "a\na\n",
            stdout: Some(""),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "13",
            args: &["-u"],
            input: "a\na\n",
            stdout: Some(""),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "20",
            args: &["-d"],
            input: "a\na\n",
            stdout: Some("a\n"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "21",
            args: &["-d"],
            input: "a\nb\n",
            stdout: Some(""),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "22",
            args: &["-d"],
            input: "a\nb\na\n",
            stdout: Some(""),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "23",
            args: &["-d"],
            input: "a\na\nb\n",
            stdout: Some("a\n"),
            stderr: None,
            exit: None,
        },
        // // Obsolete syntax for "-f 1"
        // TestCase {
        //     name: "obs30",
        //     args: &["-1"],
        //     input: "a a\nb a\n",
        //     stdout: Some("a a\n"),
        //     stderr: None,
        //     exit: None,
        // },
        TestCase {
            name: "31",
            args: &["-f", "1"],
            input: "a a\nb a\n",
            stdout: Some("a a\n"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "32",
            args: &["-f", "1"],
            input: "a a\nb b\n",
            stdout: Some("a a\nb b\n"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "33",
            args: &["-f", "1"],
            input: "a a a\nb a c\n",
            stdout: Some("a a a\nb a c\n"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "34",
            args: &["-f", "1"],
            input: "b a\na a\n",
            stdout: Some("b a\n"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "35",
            args: &["-f", "2"],
            input: "a a c\nb a c\n",
            stdout: Some("a a c\n"),
            stderr: None,
            exit: None,
        },
        // // Obsolete syntax for "-s 1"
        // TestCase {
        //     name: "obs-plus40",
        //     args: &["+1"],
        //     input: "aaa\naaa\n",
        //     stdout: Some("aaa\n"),
        //     stderr: None,
        //     exit: None,
        // },
        // TestCase {
        //     name: "obs-plus41",
        //     args: &["+1"],
        //     input: "baa\naaa\n",
        //     stdout: Some("baa\n"),
        //     stderr: None,
        //     exit: None,
        // },
        TestCase {
            name: "42",
            args: &["-s", "1"],
            input: "aaa\naaa\n",
            stdout: Some("aaa\n"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "43",
            args: &["-s", "2"],
            input: "baa\naaa\n",
            stdout: Some("baa\n"),
            stderr: None,
            exit: None,
        },
        // // Obsolete syntax for "-s 1"
        // TestCase {
        //     name: "obs-plus44",
        //     args: &["+1", "--"],
        //     input: "aaa\naaa\n",
        //     stdout: Some("aaa\n"),
        //     stderr: None,
        //     exit: None,
        // },
        // TestCase {
        //     name: "obs-plus45",
        //     args: &["+1", "--"],
        //     input: "baa\naaa\n",
        //     stdout: Some("baa\n"),
        //     stderr: None,
        //     exit: None,
        // },
        TestCase {
            name: "50",
            args: &["-f", "1", "-s", "1"],
            input: "a aaa\nb ab\n",
            stdout: Some("a aaa\nb ab\n"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "51",
            args: &["-f", "1", "-s", "1"],
            input: "a aaa\nb aaa\n",
            stdout: Some("a aaa\n"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "52",
            args: &["-s", "1", "-f", "1"],
            input: "a aaa\nb ab\n",
            stdout: Some("a aaa\nb ab\n"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "53",
            args: &["-s", "1", "-f", "1"],
            input: "a aaa\nb aaa\n",
            stdout: Some("a aaa\n"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "54",
            args: &["-s", "4"],
            input: "abc\nabcd\n",
            stdout: Some("abc\n"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "55",
            args: &["-s", "0"],
            input: "abc\nabcd\n",
            stdout: Some("abc\nabcd\n"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "56",
            args: &["-s", "0"],
            input: "abc\n",
            stdout: Some("abc\n"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "57",
            args: &["-w", "0"],
            input: "abc\nabcd\n",
            stdout: Some("abc\n"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "60",
            args: &["-w", "1"],
            input: "a a\nb a\n",
            stdout: Some("a a\nb a\n"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "61",
            args: &["-w", "3"],
            input: "a a\nb a\n",
            stdout: Some("a a\nb a\n"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "62",
            args: &["-w", "1", "-f", "1"],
            input: "a a a\nb a c\n",
            stdout: Some("a a a\n"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "63",
            args: &["-f", "1", "-w", "1"],
            input: "a a a\nb a c\n",
            stdout: Some("a a a\n"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "64",
            args: &["-f", "1", "-w", "4"],
            input: "a a a\nb a c\n",
            stdout: Some("a a a\nb a c\n"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "65",
            args: &["-f", "1", "-w", "3"],
            input: "a a a\nb a c\n",
            stdout: Some("a a a\n"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "90",
            args: &[],
            input: "a\0a\na\n",
            stdout: Some("a\0a\na\n"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "91",
            args: &[],
            input: "a\ta\na a\n",
            stdout: Some("a\ta\na a\n"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "92",
            args: &["-f", "1"],
            input: "a\ta\na a\n",
            stdout: Some("a\ta\na a\n"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "93",
            args: &["-f", "2"],
            input: "a\ta a\na a a\n",
            stdout: Some("a\ta a\n"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "94",
            args: &["-f", "1"],
            input: "a\ta\na\ta\n",
            stdout: Some("a\ta\n"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "101",
            args: &["-c"],
            input: "a\nb\n",
            stdout: Some("      1 a\n      1 b\n"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "102",
            args: &["-c"],
            input: "a\na\n",
            stdout: Some("      2 a\n"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "110",
            args: &["-D"],
            input: "a\na\n",
            stdout: Some("a\na\n"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "111",
            args: &["-D", "-w1"],
            input: "a a\na b\n",
            stdout: Some("a a\na b\n"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "112",
            args: &["-D", "-c"],
            input: "a a\na b\n",
            stdout: Some(""),
            stderr: Some("uniq: printing all duplicated lines and repeat counts is meaningless"),
            exit: Some(1),
        },
        TestCase {
            name: "113",
            args: &["--all-repeated=separate"],
            input: "a\na\n",
            stdout: Some("a\na\n"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "114",
            args: &["--all-repeated=separate"],
            input: "a\na\nb\nc\nc\n",
            stdout: Some("a\na\n\nc\nc\n"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "115",
            args: &["--all-repeated=separate"],
            input: "a\na\nb\nb\nc\n",
            stdout: Some("a\na\n\nb\nb\n"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "116",
            args: &["--all-repeated=prepend"],
            input: "a\na\n",
            stdout: Some("\na\na\n"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "117",
            args: &["--all-repeated=prepend"],
            input: "a\na\nb\nc\nc\n",
            stdout: Some("\na\na\n\nc\nc\n"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "118",
            args: &["--all-repeated=prepend"],
            input: "a\nb\n",
            stdout: Some(""),
            stderr: None,
            exit: None,
        },
        // \x08 is the backspace char
        TestCase {
            name: "120",
            args: &["-d", "-u"],
            input: "a\na\n\x08",
            stdout: Some(""),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "123",
            args: &["--zero-terminated"],
            input: "a\na\nb",
            stdout: Some("a\na\nb\0"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "124",
            args: &["--zero-terminated"],
            input: "a\0a\0b",
            stdout: Some("a\0b\0"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "125",
            args: &[],
            input: "A\na\n",
            stdout: Some("A\na\n"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "126",
            args: &["-i"],
            input: "A\na\n",
            stdout: Some("A\n"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "127",
            args: &["--ignore-case"],
            input: "A\na\n",
            stdout: Some("A\n"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "128",
            args: &["--group=prepend"],
            input: "a\na\nb\n",
            stdout: Some("\na\na\n\nb\n"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "129",
            args: &["--group=append"],
            input: "a\na\nb\n",
            stdout: Some("a\na\n\nb\n\n"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "130",
            args: &["--group=separate"],
            input: "a\na\nb\n",
            stdout: Some("a\na\n\nb\n"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "131",
            args: &["--group"],
            input: "a\na\nb\n",
            stdout: Some("a\na\n\nb\n"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "132",
            args: &["--group=both"],
            input: "a\na\nb\n",
            stdout: Some("\na\na\n\nb\n\n"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "133",
            args: &["--group=prepend"],
            input: "a\na\n",
            stdout: Some("\na\na\n"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "134",
            args: &["--group=append"],
            input: "a\na\n",
            stdout: Some("a\na\n\n"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "135",
            args: &["--group=separate"],
            input: "a\na\n",
            stdout: Some("a\na\n"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "136",
            args: &["--group"],
            input: "a\na\n",
            stdout: Some("a\na\n"),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "137",
            args: &["--group=prepend"],
            input: "",
            stdout: Some(""),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "138",
            args: &["--group=append"],
            input: "",
            stdout: Some(""),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "139",
            args: &["--group=separate"],
            input: "",
            stdout: Some(""),
            stderr: None,
            exit: None,
        },
        TestCase {
            name: "140",
            args: &["--group=both"],
            input: "",
            stdout: Some(""),
            stderr: None,
            exit: None,
        },
    ];

    for case in cases {
        eprintln!("Test {}", case.name);
        let result = new_ucmd!().args(case.args).run_piped_stdin(case.input);
        if let Some(stdout) = case.stdout {
            result.stdout_is(stdout);
        }
        if let Some(stderr) = case.stderr {
            result.stderr_contains(stderr);
        }
        if let Some(exit) = case.exit {
            result.code_is(exit);
        }
    }
}
