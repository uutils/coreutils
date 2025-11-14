// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//
// spell-checker:ignore binvalid finvalid hinvalid iinvalid linvalid nabcabc nabcabcabc ninvalid vinvalid winvalid dabc näää
use uutests::{at_and_ucmd, new_ucmd, util::TestScenario, util_name};

#[test]
#[cfg(target_os = "linux")]
fn test_non_utf8_paths() {
    use std::os::unix::ffi::OsStringExt;
    let (at, mut ucmd) = at_and_ucmd!();

    let filename = std::ffi::OsString::from_vec(vec![0xFF, 0xFE]);
    std::fs::write(at.plus(&filename), b"line 1\nline 2\nline 3\n").unwrap();

    ucmd.arg(&filename)
        .succeeds()
        .stdout_contains("1\t")
        .stdout_contains("2\t")
        .stdout_contains("3\t");
}

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails_with_code(1);
}

#[test]
fn test_stdin_no_newline() {
    new_ucmd!()
        .pipe_in("No Newline")
        .succeeds()
        .stdout_is("     1\tNo Newline\n");
}

#[test]
fn test_stdin_newline() {
    new_ucmd!()
        .args(&["-s", "-", "-w", "1"])
        .pipe_in("Line One\nLine Two\n")
        .succeeds()
        .stdout_is("1-Line One\n2-Line Two\n");
}

#[test]
fn test_padding_without_overflow() {
    new_ucmd!()
        .args(&["-i", "1000", "-s", "x", "-n", "rz", "simple.txt"])
        .succeeds()
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
        .succeeds()
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
            .succeeds()
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
#[cfg(target_os = "linux")]
fn test_number_separator_non_utf8() {
    use std::{
        ffi::{OsStr, OsString},
        os::unix::ffi::{OsStrExt, OsStringExt},
    };

    let separator_bytes = [0xFF, 0xFE];
    let mut v = b"--number-separator=".to_vec();
    v.extend_from_slice(&separator_bytes);

    let arg = OsString::from_vec(v);
    let separator = OsStr::from_bytes(&separator_bytes);

    new_ucmd!()
        .arg(arg)
        .pipe_in("test")
        .succeeds()
        .stdout_is(format!("     1{}test\n", separator.to_string_lossy()));
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
fn test_join_blank_lines_zero() {
    for arg in ["-l0", "--join-blank-lines=0"] {
        new_ucmd!()
            .arg(arg)
            .arg("--body-numbering=a")
            .pipe_in("\n\n\n\n\n\n")
            .succeeds()
            .stdout_is(concat!(
                "     1\t\n",
                "     2\t\n",
                "     3\t\n",
                "     4\t\n",
                "     5\t\n",
                "     6\t\n",
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
#[cfg(target_os = "linux")]
fn test_section_delimiter_non_utf8() {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};

    fn create_arg(prefix: &[u8]) -> OsString {
        let section_delimiter = [0xFF, 0xFE];
        let mut v = prefix.to_vec();
        v.extend_from_slice(&section_delimiter);
        OsString::from_vec(v)
    }

    let short = create_arg(b"-d");
    let long = create_arg(b"--section-delimiter=");

    for arg in [short, long] {
        let header_section: Vec<u8> =
            vec![b'a', b'\n', 0xFF, 0xFE, 0xFF, 0xFE, 0xFF, 0xFE, b'\n', b'b'];

        new_ucmd!()
            .arg(&arg)
            .pipe_in(header_section)
            .succeeds()
            .stdout_is("     1\ta\n\n       b\n");

        let body_section: Vec<u8> = vec![b'a', b'\n', 0xFF, 0xFE, 0xFF, 0xFE, b'\n', b'b'];

        new_ucmd!()
            .arg(&arg)
            .pipe_in(body_section)
            .succeeds()
            .stdout_is("     1\ta\n\n     1\tb\n");

        let footer_section: Vec<u8> = vec![b'a', b'\n', 0xFF, 0xFE, b'\n', b'b'];

        new_ucmd!()
            .arg(&arg)
            .pipe_in(footer_section)
            .succeeds()
            .stdout_is("     1\ta\n\n       b\n");
    }
}

#[test]
fn test_one_char_section_delimiter() {
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
#[cfg(target_os = "linux")]
fn test_one_byte_section_delimiter() {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};

    fn create_arg(prefix: &[u8]) -> OsString {
        let mut v = prefix.to_vec();
        v.push(0xFF);
        OsString::from_vec(v)
    }

    let short = create_arg(b"-d");
    let long = create_arg(b"--section-delimiter=");

    for arg in [short, long] {
        let header_section: Vec<u8> =
            vec![b'a', b'\n', 0xFF, b':', 0xFF, b':', 0xFF, b':', b'\n', b'b'];

        new_ucmd!()
            .arg(&arg)
            .pipe_in(header_section)
            .succeeds()
            .stdout_is("     1\ta\n\n       b\n");

        let body_section: Vec<u8> = vec![b'a', b'\n', 0xFF, b':', 0xFF, b':', b'\n', b'b'];

        new_ucmd!()
            .arg(&arg)
            .pipe_in(body_section)
            .succeeds()
            .stdout_is("     1\ta\n\n     1\tb\n");

        let footer_section: Vec<u8> = vec![b'a', b'\n', 0xFF, b':', b'\n', b'b'];

        new_ucmd!()
            .arg(&arg)
            .pipe_in(footer_section)
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

#[test]
fn test_file_with_non_utf8_content() {
    let (at, mut ucmd) = at_and_ucmd!();

    let filename = "file";
    let content: &[u8] = b"a\n\xFF\xFE\nb";
    let invalid_utf8: &[u8] = b"\xFF\xFE";

    at.write_bytes(filename, content);

    ucmd.arg(filename).succeeds().stdout_is(format!(
        "     1\ta\n     2\t{}\n     3\tb\n",
        String::from_utf8_lossy(invalid_utf8)
    ));
}

// Regression tests for issue #9132: repeated flags should use last value
#[test]
fn test_repeated_body_numbering_flag() {
    // -ba -bt should use -bt (t=nonempty)
    new_ucmd!()
        .args(&["-ba", "-bt"])
        .pipe_in("a\n\nb\n\nc")
        .succeeds()
        .stdout_is("     1\ta\n       \n     2\tb\n       \n     3\tc\n");
}

#[test]
fn test_repeated_header_numbering_flag() {
    // -ha -ht should use -ht (number only nonempty lines in header)
    new_ucmd!()
        .args(&["-ha", "-ht"])
        .pipe_in("\\:\\:\\:\na\nb\n\nc")
        .succeeds()
        .stdout_is("\n     1\ta\n     2\tb\n       \n     3\tc\n");
}

#[test]
fn test_repeated_footer_numbering_flag() {
    // -fa -ft should use -ft (t=nonempty in footer)
    new_ucmd!()
        .args(&["-fa", "-ft"])
        .pipe_in("\\:\na\nb\n\nc")
        .succeeds()
        .stdout_is("\n     1\ta\n     2\tb\n       \n     3\tc\n");
}

#[test]
fn test_repeated_number_format_flag() {
    // -n ln -n rn should use -n rn (rn=right aligned)
    new_ucmd!()
        .args(&["-n", "ln", "-n", "rn"])
        .pipe_in("a\nb\nc")
        .succeeds()
        .stdout_is("     1\ta\n     2\tb\n     3\tc\n");
}

#[test]
fn test_repeated_number_separator_flag() {
    // -s ':' -s '|' should use -s '|'
    new_ucmd!()
        .args(&["-s", ":", "-s", "|"])
        .pipe_in("a\nb\nc")
        .succeeds()
        .stdout_is("     1|a\n     2|b\n     3|c\n");
}

#[test]
fn test_repeated_number_width_flag() {
    // -w 3 -w 8 should use -w 8
    new_ucmd!()
        .args(&["-w", "3", "-w", "8"])
        .pipe_in("a\nb\nc")
        .succeeds()
        .stdout_is("       1\ta\n       2\tb\n       3\tc\n");
}

#[test]
fn test_repeated_line_increment_flag() {
    // -i 1 -i 5 should use -i 5
    new_ucmd!()
        .args(&["-i", "1", "-i", "5"])
        .pipe_in("a\nb\nc")
        .succeeds()
        .stdout_is("     1\ta\n     6\tb\n    11\tc\n");
}

#[test]
fn test_repeated_starting_line_number_flag() {
    // -v 1 -v 10 should use -v 10
    new_ucmd!()
        .args(&["-v", "1", "-v", "10"])
        .pipe_in("a\nb\nc")
        .succeeds()
        .stdout_is("    10\ta\n    11\tb\n    12\tc\n");
}

#[test]
fn test_repeated_join_blank_lines_flag() {
    // -l 1 -l 2 should use -l 2
    new_ucmd!()
        .args(&["-l", "1", "-l", "2", "-ba"])
        .pipe_in("a\n\n\nb")
        .succeeds()
        .stdout_is("     1\ta\n       \n     2\t\n     3\tb\n");
}

#[test]
fn test_repeated_section_delimiter_flag() {
    // -d ':' -d '|' should use -d '|'
    new_ucmd!()
        .args(&["-d", ":", "-d", "|"])
        .pipe_in("|:|:|:\na\nb\nc")
        .succeeds()
        .stdout_is("\n       a\n       b\n       c\n");
}
