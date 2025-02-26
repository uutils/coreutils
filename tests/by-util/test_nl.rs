// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//
// spell-checker:ignore binvalid finvalid hinvalid iinvalid linvalid nabcabc nabcabcabc ninvalid vinvalid winvalid dabc näää
use uutests::at_and_ucmd;
use uutests::new_ucmd;
use uutests::util::TestScenario;
use uutests::util_name;

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails().code_is(1);
}

#[test]
fn test_stdin_no_newline() {
    new_ucmd!()
        .pipe_in("No Newline")
        .run()
        .stdout_is("     1\tNo Newline\n");
}

#[test]
fn test_stdin_newline() {
    new_ucmd!()
        .args(&["-s", "-", "-w", "1"])
        .pipe_in("Line One\nLine Two\n")
        .run()
        .stdout_is("1-Line One\n2-Line Two\n");
}

#[test]
fn test_padding_without_overflow() {
    new_ucmd!()
        .args(&["-i", "1000", "-s", "x", "-n", "rz", "simple.txt"])
        .run()
        .stdout_is(
            "000001xL1\n001001xL2\n002001xL3\n003001xL4\n004001xL5\n005001xL6\n006001xL7\n0070\
             01xL8\n008001xL9\n009001xL10\n010001xL11\n011001xL12\n012001xL13\n013001xL14\n014\
             001xL15\n",
        );
}

#[test]
fn test_padding_with_overflow() {
    new_ucmd!()
        .args(&["-i", "1000", "-s", "x", "-n", "rz", "-w", "4", "simple.txt"])
        .run()
        .stdout_is(
            "0001xL1\n1001xL2\n2001xL3\n3001xL4\n4001xL5\n5001xL6\n6001xL7\n7001xL8\n8001xL9\n\
             9001xL10\n10001xL11\n11001xL12\n12001xL13\n13001xL14\n14001xL15\n",
        );
}

#[test]
fn test_sections_and_styles() {
    // spell-checker:disable
    for (fixture, output) in [
        (
            "section.txt",
            "\n    HEADER1\n    HEADER2\n\n1  |BODY1\n2  \
             |BODY2\n\n    FOOTER1\n    FOOTER2\n\n    NEXTHEADER1\n    NEXTHEADER2\n\n1  \
             |NEXTBODY1\n2  |NEXTBODY2\n\n    NEXTFOOTER1\n    NEXTFOOTER2\n",
        ),
        (
            "joinblanklines.txt",
            "1  |Nonempty\n2  |Nonempty\n3  |Followed by 10x empty\n    \n    \n    \n    \n4  \
             |\n    \n    \n    \n    \n5  |\n6  |Followed by 5x empty\n    \n    \n    \n    \n7  |\n8  \
             |Followed by 4x empty\n    \n    \n    \n    \n9  |Nonempty\n10 |Nonempty\n11 \
             |Nonempty.\n",
        ),
    ] {
        new_ucmd!()
            .args(&[
                "-s", "|", "-n", "ln", "-w", "3", "-b", "a", "-l", "5", fixture,
            ])
            .run()
            .stdout_is(output);
    }
    // spell-checker:enable
}

#[test]
fn test_no_renumber() {
    for arg in ["-p", "--no-renumber"] {
        new_ucmd!()
            .arg(arg)
            .pipe_in("a\n\\:\\:\nb")
            .succeeds()
            .stdout_is("     1\ta\n\n     2\tb\n");
    }
}

#[test]
fn test_number_format_ln() {
    for arg in ["-nln", "--number-format=ln"] {
        new_ucmd!()
            .arg(arg)
            .pipe_in("test")
            .succeeds()
            .stdout_is("1     \ttest\n");
    }
}

#[test]
fn test_number_format_rn() {
    for arg in ["-nrn", "--number-format=rn"] {
        new_ucmd!()
            .arg(arg)
            .pipe_in("test")
            .succeeds()
            .stdout_is("     1\ttest\n");
    }
}

#[test]
fn test_number_format_rz() {
    for arg in ["-nrz", "--number-format=rz"] {
        new_ucmd!()
            .arg(arg)
            .pipe_in("test")
            .succeeds()
            .stdout_is("000001\ttest\n");
    }
}

#[test]
fn test_number_format_rz_with_negative_line_number() {
    for arg in ["-nrz", "--number-format=rz"] {
        new_ucmd!()
            .arg(arg)
            .arg("-v-12")
            .pipe_in("test")
            .succeeds()
            .stdout_is("-00012\ttest\n");
    }
}

#[test]
fn test_invalid_number_format() {
    for arg in ["-ninvalid", "--number-format=invalid"] {
        new_ucmd!()
            .arg(arg)
            .fails()
            .stderr_contains("invalid value 'invalid'");
    }
}

#[test]
fn test_number_width() {
    for width in 1..10 {
        for arg in [format!("-w{width}"), format!("--number-width={width}")] {
            let spaces = " ".repeat(width - 1);
            new_ucmd!()
                .arg(arg)
                .pipe_in("test")
                .succeeds()
                .stdout_is(format!("{spaces}1\ttest\n"));
        }
    }
}

#[test]
fn test_number_width_zero() {
    for arg in ["-w0", "--number-width=0"] {
        new_ucmd!()
            .arg(arg)
            .fails()
            .stderr_contains("Invalid line number field width: ‘0’: Numerical result out of range");
    }
}

#[test]
fn test_invalid_number_width() {
    for arg in ["-winvalid", "--number-width=invalid"] {
        new_ucmd!()
            .arg(arg)
            .fails()
            .stderr_contains("invalid value 'invalid'");
    }
}

#[test]
fn test_number_separator() {
    for arg in ["-s:-:", "--number-separator=:-:"] {
        new_ucmd!()
            .arg(arg)
            .pipe_in("test")
            .succeeds()
            .stdout_is("     1:-:test\n");
    }
}

#[test]
fn test_starting_line_number() {
    for arg in ["-v10", "--starting-line-number=10"] {
        new_ucmd!()
            .arg(arg)
            .pipe_in("test")
            .succeeds()
            .stdout_is("    10\ttest\n");
    }
}

#[test]
fn test_negative_starting_line_number() {
    for arg in ["-v-10", "--starting-line-number=-10"] {
        new_ucmd!()
            .arg(arg)
            .pipe_in("test")
            .succeeds()
            .stdout_is("   -10\ttest\n");
    }
}

#[test]
fn test_invalid_starting_line_number() {
    for arg in ["-vinvalid", "--starting-line-number=invalid"] {
        new_ucmd!()
            .arg(arg)
            .fails()
            .stderr_contains("invalid value 'invalid'");
    }
}

#[test]
fn test_line_increment() {
    for arg in ["-i10", "--line-increment=10"] {
        new_ucmd!()
            .arg(arg)
            .pipe_in("a\nb")
            .succeeds()
            .stdout_is("     1\ta\n    11\tb\n");
    }
}

#[test]
fn test_line_increment_from_negative_starting_line() {
    for arg in ["-i10", "--line-increment=10"] {
        new_ucmd!()
            .arg(arg)
            .arg("-v-19")
            .pipe_in("a\nb\nc")
            .succeeds()
            .stdout_is("   -19\ta\n    -9\tb\n     1\tc\n");
    }
}

#[test]
fn test_negative_line_increment() {
    for arg in ["-i-10", "--line-increment=-10"] {
        new_ucmd!()
            .arg(arg)
            .pipe_in("a\nb\nc")
            .succeeds()
            .stdout_is("     1\ta\n    -9\tb\n   -19\tc\n");
    }
}

#[test]
fn test_invalid_line_increment() {
    for arg in ["-iinvalid", "--line-increment=invalid"] {
        new_ucmd!()
            .arg(arg)
            .fails()
            .stderr_contains("invalid value 'invalid'");
    }
}

#[test]
fn test_join_blank_lines() {
    for arg in ["-l3", "--join-blank-lines=3"] {
        new_ucmd!()
            .arg(arg)
            .arg("--body-numbering=a")
            .pipe_in("\n\n\n\n\n\n")
            .succeeds()
            .stdout_is(concat!(
                "       \n",
                "       \n",
                "     1\t\n",
                "       \n",
                "       \n",
                "     2\t\n",
            ));
    }
}

#[test]
fn test_join_blank_lines_multiple_files() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.write("a.txt", "\n\n");
    at.write("b.txt", "\n\n");
    at.write("c.txt", "\n\n");

    for arg in ["-l3", "--join-blank-lines=3"] {
        scene
            .ucmd()
            .args(&[arg, "--body-numbering=a", "a.txt", "b.txt", "c.txt"])
            .succeeds()
            .stdout_is(concat!(
                "       \n",
                "       \n",
                "     1\t\n",
                "       \n",
                "       \n",
                "     2\t\n",
            ));
    }
}

#[test]
fn test_join_blank_lines_zero() {
    for arg in ["-l0", "--join-blank-lines=0"] {
        new_ucmd!().arg(arg).fails().stderr_contains(
            "Invalid line number of blank lines: ‘0’: Numerical result out of range",
        );
    }
}

#[test]
fn test_invalid_join_blank_lines() {
    for arg in ["-linvalid", "--join-blank-lines=invalid"] {
        new_ucmd!()
            .arg(arg)
            .fails()
            .stderr_contains("invalid value 'invalid'");
    }
}

#[test]
fn test_default_body_numbering() {
    new_ucmd!()
        .pipe_in("a\n\nb")
        .succeeds()
        .stdout_is("     1\ta\n       \n     2\tb\n");
}

#[test]
fn test_default_body_numbering_multiple_files() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.write("a.txt", "a");
    at.write("b.txt", "b");
    at.write("c.txt", "c");

    ucmd.args(&["a.txt", "b.txt", "c.txt"])
        .succeeds()
        .stdout_is("     1\ta\n     2\tb\n     3\tc\n");
}

#[test]
fn test_default_body_numbering_multiple_files_and_stdin() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.write("a.txt", "a");
    at.write("c.txt", "c");

    ucmd.args(&["a.txt", "-", "c.txt"])
        .pipe_in("b")
        .succeeds()
        .stdout_is("     1\ta\n     2\tb\n     3\tc\n");
}

#[test]
fn test_body_numbering_all_lines_without_delimiter() {
    for arg in ["-ba", "--body-numbering=a"] {
        new_ucmd!()
            .arg(arg)
            .pipe_in("a\n\nb")
            .succeeds()
            .stdout_is("     1\ta\n     2\t\n     3\tb\n");
    }
}

#[test]
fn test_body_numbering_no_lines_without_delimiter() {
    for arg in ["-bn", "--body-numbering=n"] {
        new_ucmd!()
            .arg(arg)
            .pipe_in("a\n\nb")
            .succeeds()
            .stdout_is("       a\n       \n       b\n");
    }
}

#[test]
fn test_body_numbering_non_empty_lines_without_delimiter() {
    for arg in ["-bt", "--body-numbering=t"] {
        new_ucmd!()
            .arg(arg)
            .pipe_in("a\n\nb")
            .succeeds()
            .stdout_is("     1\ta\n       \n     2\tb\n");
    }
}

#[test]
fn test_body_numbering_matched_lines_without_delimiter() {
    for arg in ["-bp^[ac]", "--body-numbering=p^[ac]"] {
        new_ucmd!()
            .arg(arg)
            .pipe_in("a\nb\nc")
            .succeeds()
            .stdout_is("     1\ta\n       b\n     2\tc\n");
    }
}

#[test]
fn test_numbering_all_lines() {
    let delimiters_and_args = [
        ("\\:\\:\\:\n", ["-ha", "--header-numbering=a"]),
        ("\\:\\:\n", ["-ba", "--body-numbering=a"]),
        ("\\:\n", ["-fa", "--footer-numbering=a"]),
    ];

    for (delimiter, args) in delimiters_and_args {
        for arg in args {
            new_ucmd!()
                .arg(arg)
                .pipe_in(format!("{delimiter}a\n\nb"))
                .succeeds()
                .stdout_is("\n     1\ta\n     2\t\n     3\tb\n");
        }
    }
}

#[test]
fn test_numbering_no_lines() {
    let delimiters_and_args = [
        ("\\:\\:\\:\n", ["-hn", "--header-numbering=n"]),
        ("\\:\\:\n", ["-bn", "--body-numbering=n"]),
        ("\\:\n", ["-fn", "--footer-numbering=n"]),
    ];

    for (delimiter, args) in delimiters_and_args {
        for arg in args {
            new_ucmd!()
                .arg(arg)
                .pipe_in(format!("{delimiter}a\n\nb"))
                .succeeds()
                .stdout_is("\n       a\n       \n       b\n");
        }
    }
}

#[test]
fn test_numbering_non_empty_lines() {
    let delimiters_and_args = [
        ("\\:\\:\\:\n", ["-ht", "--header-numbering=t"]),
        ("\\:\\:\n", ["-bt", "--body-numbering=t"]),
        ("\\:\n", ["-ft", "--footer-numbering=t"]),
    ];

    for (delimiter, args) in delimiters_and_args {
        for arg in args {
            new_ucmd!()
                .arg(arg)
                .pipe_in(format!("{delimiter}a\n\nb"))
                .succeeds()
                .stdout_is("\n     1\ta\n       \n     2\tb\n");
        }
    }
}

#[test]
fn test_numbering_matched_lines() {
    let delimiters_and_args = [
        ("\\:\\:\\:\n", ["-hp^[ac]", "--header-numbering=p^[ac]"]),
        ("\\:\\:\n", ["-bp^[ac]", "--body-numbering=p^[ac]"]),
        ("\\:\n", ["-fp^[ac]", "--footer-numbering=p^[ac]"]),
    ];

    for (delimiter, args) in delimiters_and_args {
        for arg in args {
            new_ucmd!()
                .arg(arg)
                .pipe_in(format!("{delimiter}a\nb\nc"))
                .succeeds()
                .stdout_is("\n     1\ta\n       b\n     2\tc\n");
        }
    }
}

#[test]
fn test_invalid_numbering() {
    let invalid_args = [
        "-hinvalid",
        "--header-numbering=invalid",
        "-binvalid",
        "--body-numbering=invalid",
        "-finvalid",
        "--footer-numbering=invalid",
    ];

    for invalid_arg in invalid_args {
        new_ucmd!()
            .arg(invalid_arg)
            .fails()
            .stderr_contains("invalid numbering style: 'invalid'");
    }
}

#[test]
fn test_invalid_regex_numbering() {
    let invalid_args = [
        "-hp[",
        "--header-numbering=p[",
        "-bp[",
        "--body-numbering=p[",
        "-fp[",
        "--footer-numbering=p[",
    ];

    for invalid_arg in invalid_args {
        new_ucmd!()
            .arg(invalid_arg)
            .fails()
            .stderr_contains("invalid regular expression");
    }
}

#[test]
fn test_line_number_overflow() {
    new_ucmd!()
        .arg(format!("--starting-line-number={}", i64::MAX))
        .pipe_in("a\nb")
        .fails()
        .stdout_is(format!("{}\ta\n", i64::MAX))
        .stderr_is("nl: line number overflow\n");

    new_ucmd!()
        .arg(format!("--starting-line-number={}", i64::MIN))
        .arg("--line-increment=-1")
        .pipe_in("a\nb")
        .fails()
        .stdout_is(format!("{}\ta\n", i64::MIN))
        .stderr_is("nl: line number overflow\n");
}

#[test]
fn test_line_number_no_overflow() {
    new_ucmd!()
        .arg(format!("--starting-line-number={}", i64::MAX))
        .pipe_in("a\n\\:\\:\nb")
        .succeeds()
        .stdout_is(format!("{0}\ta\n\n{0}\tb\n", i64::MAX));

    new_ucmd!()
        .arg(format!("--starting-line-number={}", i64::MIN))
        .arg("--line-increment=-1")
        .pipe_in("a\n\\:\\:\nb")
        .succeeds()
        .stdout_is(format!("{0}\ta\n\n{0}\tb\n", i64::MIN));
}

#[test]
fn test_section_delimiter() {
    for arg in ["-dabc", "--section-delimiter=abc"] {
        new_ucmd!()
            .arg(arg)
            .pipe_in("a\nabcabcabc\nb") // header section
            .succeeds()
            .stdout_is("     1\ta\n\n       b\n");

        new_ucmd!()
            .arg(arg)
            .pipe_in("a\nabcabc\nb") // body section
            .succeeds()
            .stdout_is("     1\ta\n\n     1\tb\n");

        new_ucmd!()
            .arg(arg)
            .pipe_in("a\nabc\nb") // footer section
            .succeeds()
            .stdout_is("     1\ta\n\n       b\n");
    }
}

#[test]
fn test_one_char_section_delimiter_expansion() {
    for arg in ["-da", "--section-delimiter=a"] {
        new_ucmd!()
            .arg(arg)
            .pipe_in("a\na:a:a:\nb") // header section
            .succeeds()
            .stdout_is("     1\ta\n\n       b\n");

        new_ucmd!()
            .arg(arg)
            .pipe_in("a\na:a:\nb") // body section
            .succeeds()
            .stdout_is("     1\ta\n\n     1\tb\n");

        new_ucmd!()
            .arg(arg)
            .pipe_in("a\na:\nb") // footer section
            .succeeds()
            .stdout_is("     1\ta\n\n       b\n");
    }
}

#[test]
fn test_non_ascii_one_char_section_delimiter() {
    for arg in ["-dä", "--section-delimiter=ä"] {
        new_ucmd!()
            .arg(arg)
            .pipe_in("a\näää\nb") // header section
            .succeeds()
            .stdout_is("     1\ta\n\n       b\n");

        new_ucmd!()
            .arg(arg)
            .pipe_in("a\nää\nb") // body section
            .succeeds()
            .stdout_is("     1\ta\n\n     1\tb\n");

        new_ucmd!()
            .arg(arg)
            .pipe_in("a\nä\nb") // footer section
            .succeeds()
            .stdout_is("     1\ta\n\n       b\n");
    }
}

#[test]
fn test_empty_section_delimiter() {
    for arg in ["-d ''", "--section-delimiter=''"] {
        new_ucmd!()
            .arg(arg)
            .pipe_in("a\n\nb")
            .succeeds()
            .stdout_is("     1\ta\n       \n     2\tb\n");
    }
}

#[test]
fn test_directory_as_input() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir = "dir";
    let file = "file";
    let content = "aaa";

    at.mkdir(dir);
    at.write(file, content);

    ucmd.arg(dir)
        .arg(file)
        .fails()
        .stderr_is(format!("nl: {dir}: Is a directory\n"))
        .stdout_contains(content);
}
