// spell-checker:ignore iinvalid linvalid ninvalid vinvalid winvalid
use crate::common::util::TestScenario;

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
            "\nHEADER1\nHEADER2\n\n1  |BODY1\n2  \
             |BODY2\n\nFOOTER1\nFOOTER2\n\nNEXTHEADER1\nNEXTHEADER2\n\n1  \
             |NEXTBODY1\n2  |NEXTBODY2\n\nNEXTFOOTER1\nNEXTFOOTER2\n",
        ),
        (
            "joinblanklines.txt",
            "1  |Nonempty\n2  |Nonempty\n3  |Followed by 10x empty\n\n\n\n\n4  \
             |\n\n\n\n\n5  |\n6  |Followed by 5x empty\n\n\n\n\n7  |\n8  \
             |Followed by 4x empty\n\n\n\n\n9  |Nonempty\n10 |Nonempty\n11 \
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
        new_ucmd!().arg(arg).succeeds();
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
fn test_header_numbering_all_lines() {
    for arg in ["-ha", "--header-numbering=a"] {
        new_ucmd!()
            .arg(arg)
            .pipe_in("\\:\\:\\:\na\n\nb")
            .succeeds()
            .stdout_is("\n     1\ta\n     2\t\n     3\tb\n");
    }
}

#[test]
fn test_body_numbering_all_lines() {
    for arg in ["-ba", "--body-numbering=a"] {
        new_ucmd!()
            .arg(arg)
            .pipe_in("\\:\\:\na\n\nb")
            .succeeds()
            .stdout_is("\n     1\ta\n     2\t\n     3\tb\n");
    }
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
fn test_footer_numbering_all_lines() {
    for arg in ["-fa", "--footer-numbering=a"] {
        new_ucmd!()
            .arg(arg)
            .pipe_in("\\:\na\n\nb")
            .succeeds()
            .stdout_is("\n     1\ta\n     2\t\n     3\tb\n");
    }
}
