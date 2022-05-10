use crate::common::util::*;

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
