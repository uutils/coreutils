// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore bsdutils toybox

use uutests::at_and_ucmd;
use uutests::new_ucmd;
use uutests::util::TestScenario;
use uutests::util_name;

struct TestData<'b> {
    name: &'b str,
    args: &'b [&'b str],
    ins: &'b [&'b str],
    out: &'b str,
}

const EXAMPLE_DATA: &[TestData] = &[
    // Ensure that paste properly handles files lacking a final newline.
    TestData {
        name: "no-nl-1",
        args: &[],
        ins: &["a", "b"],
        out: "a\tb\n",
    },
    TestData {
        name: "no-nl-2",
        args: &[],
        ins: &["a\n", "b"],
        out: "a\tb\n",
    },
    TestData {
        name: "no-nl-3",
        args: &[],
        ins: &["a", "b\n"],
        out: "a\tb\n",
    },
    TestData {
        name: "no-nl-4",
        args: &[],
        ins: &["a\n", "b\n"],
        out: "a\tb\n",
    },
    TestData {
        name: "zno-nl-1",
        args: &["-z"],
        ins: &["a", "b"],
        out: "a\tb\0",
    },
    TestData {
        name: "zno-nl-2",
        args: &["-z"],
        ins: &["a\0", "b"],
        out: "a\tb\0",
    },
    TestData {
        name: "zno-nl-3",
        args: &["-z"],
        ins: &["a", "b\0"],
        out: "a\tb\0",
    },
    TestData {
        name: "zno-nl-4",
        args: &["-z"],
        ins: &["a\0", "b\0"],
        out: "a\tb\0",
    },
    // Same as above, but with a two lines in each input file and the
    // addition of the -d option to make SPACE be the output
    // delimiter.
    TestData {
        name: "no-nla-1",
        args: &["-d", " "],
        ins: &["1\na", "2\nb"],
        out: "1 2\na b\n",
    },
    TestData {
        name: "no-nla-2",
        args: &["-d", " "],
        ins: &["1\na\n", "2\nb"],
        out: "1 2\na b\n",
    },
    TestData {
        name: "no-nla-3",
        args: &["-d", " "],
        ins: &["1\na", "2\nb\n"],
        out: "1 2\na b\n",
    },
    TestData {
        name: "no-nla-4",
        args: &["-d", " "],
        ins: &["1\na\n", "2\nb\n"],
        out: "1 2\na b\n",
    },
    TestData {
        name: "zno-nla1",
        args: &["-zd", " "],
        ins: &["1\0a", "2\0b"],
        out: "1 2\0a b\0",
    },
    TestData {
        name: "zno-nla2",
        args: &["-zd", " "],
        ins: &["1\0a\0", "2\0b"],
        out: "1 2\0a b\0",
    },
    TestData {
        name: "zno-nla3",
        args: &["-zd", " "],
        ins: &["1\0a", "2\0b\0"],
        out: "1 2\0a b\0",
    },
    TestData {
        name: "zno-nla4",
        args: &["-zd", " "],
        ins: &["1\0a\0", "2\0b\0"],
        out: "1 2\0a b\0",
    },
    TestData {
        name: "multibyte-delim",
        args: &["-d", "💣"],
        ins: &["1\na\n", "2\nb\n"],
        out: "1💣2\na💣b\n",
    },
    TestData {
        name: "multibyte-delim-serial",
        args: &["-d", "💣", "-s"],
        ins: &["1\na\n", "2\nb\n"],
        out: "1💣a\n2💣b\n",
    },
    TestData {
        name: "trailing whitespace",
        args: &["-d", "|"],
        ins: &["1 \na \n", "2\t\nb\t\n"],
        out: "1 |2\t\na |b\t\n",
    },
];

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails().code_is(1);
}

#[test]
fn test_combine_pairs_of_lines() {
    for s in ["-s", "--serial"] {
        for d in ["-d", "--delimiters"] {
            new_ucmd!()
                .args(&[s, d, "\t\n", "html_colors.txt"])
                .run()
                .stdout_is_fixture("html_colors.expected");
        }
    }
}

#[test]
fn test_multi_stdin() {
    for d in ["-d", "--delimiters"] {
        new_ucmd!()
            .args(&[d, "\t\n", "-", "-"])
            .pipe_in_fixture("html_colors.txt")
            .succeeds()
            .stdout_is_fixture("html_colors.expected");
    }
}

#[test]
// TODO: make this test work on Windows
#[cfg(not(windows))]
fn test_delimiter_list_ending_with_escaped_backslash() {
    for d in ["-d", "--delimiters"] {
        let (at, mut ucmd) = at_and_ucmd!();
        let mut ins = vec![];
        for (i, one_in) in ["a\n", "b\n"].iter().enumerate() {
            let file = format!("in{i}");
            at.write(&file, one_in);
            ins.push(file);
        }
        ucmd.args(&[d, r"\\"])
            .args(&ins)
            .succeeds()
            .stdout_is("a\\b\n");
    }
}

#[test]
fn test_delimiter_list_ending_with_unescaped_backslash() {
    for d in ["-d", "--delimiters"] {
        new_ucmd!()
            .args(&[d, r"\"])
            .fails()
            .stderr_contains(r"delimiter list ends with an unescaped backslash: \");

        new_ucmd!()
            .args(&[d, r"\\\"])
            .fails()
            .stderr_contains(r"delimiter list ends with an unescaped backslash: \\\");

        new_ucmd!()
            .args(&[d, r"_\"])
            .fails()
            .stderr_contains(r"delimiter list ends with an unescaped backslash: _\");
    }
}

#[test]
fn test_delimiter_list_empty() {
    for option_style in ["-d", "--delimiters"] {
        new_ucmd!()
            .args(&[option_style, "", "-s"])
            .pipe_in(
                "\
A ALPHA 1 _
B BRAVO 2 _
C CHARLIE 3 _
",
            )
            .succeeds()
            .stdout_only(
                "\
A ALPHA 1 _B BRAVO 2 _C CHARLIE 3 _
",
            );
    }
}

// Was panicking (usize subtraction that would have resulted in a negative number)
// Not observable in release builds, since integer overflow checking is not enabled
#[test]
fn test_delimiter_truncation() {
    for option_style in ["-d", "--delimiters"] {
        new_ucmd!()
            .args(&[option_style, "!@#", "-s", "-", "-", "-"])
            .pipe_in(
                "\
FIRST
SECOND
THIRD
FOURTH
ABCDEFG
",
            )
            .succeeds()
            .stdout_only(
                "\
FIRST!SECOND@THIRD#FOURTH!ABCDEFG


",
            );
    }
}

#[test]
fn test_non_utf8_input() {
    // 0xC0 is not valid UTF-8
    const INPUT: &[u8] = b"Non-UTF-8 test: \xC0\x00\xC0.\n";

    new_ucmd!()
        .pipe_in(INPUT)
        .succeeds()
        .stdout_only_bytes(INPUT);
}

#[test]
fn test_three_trailing_backslashes_delimiter() {
    const ONE_BACKSLASH_STR: &str = r"\";

    let three_backslashes_string = ONE_BACKSLASH_STR.repeat(3);

    for option_style in ["-d", "--delimiters"] {
        new_ucmd!()
            .args(&[option_style, &three_backslashes_string])
            .fails()
            .no_stdout()
            .stderr_str_check(|st| {
                st.ends_with(&format!(
                    ": delimiter list ends with an unescaped backslash: {three_backslashes_string}\n"
                ))
            });
    }
}

// "If any other characters follow the <backslash>, the results are unspecified."
// https://pubs.opengroup.org/onlinepubs/9799919799/utilities/paste.html
// However, other implementations remove the backslash
#[test]
fn test_posix_unspecified_delimiter() {
    for option_style in ["-d", "--delimiters"] {
        new_ucmd!()
            .args(&[option_style, r"\z", "-s"])
            .pipe_in(
                "\
1
2
3
4
",
            )
            .succeeds()
            .stdout_only(
                "\
1z2z3z4
",
            );
    }
}

// "Empty string (not a null character)"
// https://pubs.opengroup.org/onlinepubs/9799919799/utilities/paste.html
#[test]
fn test_backslash_zero_delimiter() {
    for option_style in ["-d", "--delimiters"] {
        new_ucmd!()
            .args(&[option_style, r"\0z\0", "-s"])
            .pipe_in(
                "\
1
2
3
4
5
6
",
            )
            .succeeds()
            .stdout_only(
                "\
12z345z6
",
            );
    }
}

// As of 2024-10-09, only bsdutils (https://github.com/dcantrell/bsdutils, derived from FreeBSD) and toybox handle
// multibyte delimiter characters in the way a user would likely expect. BusyBox and GNU Core Utilities do not.
#[test]
fn test_multi_byte_delimiter() {
    for option_style in ["-d", "--delimiters"] {
        new_ucmd!()
            .args(&[option_style, "!ß@", "-s"])
            .pipe_in(
                "\
1
2
3
4
5
6
",
            )
            .succeeds()
            .stdout_only(
                "\
1!2ß3@4!5ß6
",
            );
    }
}

#[test]
fn test_data() {
    for example in EXAMPLE_DATA {
        let (at, mut ucmd) = at_and_ucmd!();
        let mut ins = vec![];
        for (i, one_in) in example.ins.iter().enumerate() {
            let file = format!("in{i}");
            at.write(&file, one_in);
            ins.push(file);
        }
        println!("{}", example.name);
        ucmd.args(example.args)
            .args(&ins)
            .succeeds()
            .stdout_is(example.out);
    }
}
