// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use crate::common::util::TestScenario;

struct TestData<'b> {
    name: &'b str,
    args: &'b [&'b str],
    ins: &'b [&'b str],
    out: &'b str,
}

static EXAMPLE_DATA: &[TestData] = &[
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
            let file = format!("in{}", i);
            at.write(&file, one_in);
            ins.push(file);
        }
        ucmd.args(&[d, "\\\\"])
            .args(&ins)
            .succeeds()
            .stdout_is("a\\b\n");
    }
}

#[test]
fn test_delimiter_list_ending_with_unescaped_backslash() {
    for d in ["-d", "--delimiters"] {
        new_ucmd!()
            .args(&[d, "\\"])
            .fails()
            .stderr_contains("delimiter list ends with an unescaped backslash: \\");
        new_ucmd!()
            .args(&[d, "_\\"])
            .fails()
            .stderr_contains("delimiter list ends with an unescaped backslash: _\\");
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
