use std::io::process::Command;
use std::str;

static PROGNAME: &'static str = "./nl";

#[test]
fn test_stdin_nonewline() {
    let mut process = Command::new(PROGNAME).spawn().unwrap();
    process.stdin.take().unwrap().write(b"No Newline").unwrap();
    let po = process.wait_with_output().unwrap();
    let out =  str::from_utf8(po.output.as_slice()).unwrap();

    assert_eq!(out, "     1\tNo Newline\n");
}
#[test]
fn test_stdin_newline() {
    let mut process = Command::new(PROGNAME).arg("-s").arg("-")
        .arg("-w").arg("1").spawn().unwrap();

    process.stdin.take().unwrap().write(b"Line One\nLine Two\n").unwrap();
    let po = process.wait_with_output().unwrap();
    let out =  str::from_utf8(po.output.as_slice()).unwrap();

    assert_eq!(out, "1-Line One\n2-Line Two\n");
}

#[test]
fn test_padding_without_overflow() {
    let po = Command::new(PROGNAME).arg("-i").arg("1000").arg("-s").arg("x")
        .arg("-n").arg("rz").arg("simple.txt").output().unwrap();

    let out =  str::from_utf8(po.output.as_slice()).unwrap();
    assert_eq!(out, "000001xL1\n001001xL2\n002001xL3\n003001xL4\n004001xL5\n005001xL6\n006001xL7\n007001xL8\n008001xL9\n009001xL10\n010001xL11\n011001xL12\n012001xL13\n013001xL14\n014001xL15\n");
}

#[test]
fn test_padding_with_overflow() {
    let po = Command::new(PROGNAME).arg("-i").arg("1000").arg("-s").arg("x")
        .arg("-n").arg("rz").arg("-w").arg("4")
        .arg("simple.txt").output().unwrap();

    let out =  str::from_utf8(po.output.as_slice()).unwrap();
    assert_eq!(out, "0001xL1\n1001xL2\n2001xL3\n3001xL4\n4001xL5\n5001xL6\n6001xL7\n7001xL8\n8001xL9\n9001xL10\n10001xL11\n11001xL12\n12001xL13\n13001xL14\n14001xL15\n");
}

#[test]
fn test_sections_and_styles() {
    for &(fixture, output) in [
        (
            "section.txt", 
            "\nHEADER1\nHEADER2\n\n1  |BODY1\n2  |BODY2\n\nFOOTER1\nFOOTER2\n\nNEXTHEADER1\nNEXTHEADER2\n\n1  |NEXTBODY1\n2  |NEXTBODY2\n\nNEXTFOOTER1\nNEXTFOOTER2\n"
        ),
        (
            "joinblanklines.txt",
            "1  |Nonempty\n2  |Nonempty\n3  |Followed by 10x empty\n\n\n\n\n4  |\n\n\n\n\n5  |\n6  |Followed by 5x empty\n\n\n\n\n7  |\n8  |Followed by 4x empty\n\n\n\n\n9  |Nonempty\n10 |Nonempty\n11 |Nonempty.\n"
        ),
    ].iter() {
        let po = Command::new(PROGNAME).arg("-s").arg("|").arg("-n").arg("ln")
            .arg("-w").arg("3").arg("-b").arg("a").arg("-l").arg("5")
            .arg(fixture).output().unwrap();
        assert_eq!(str::from_utf8(po.output.as_slice()).unwrap(), output);
    }
}
