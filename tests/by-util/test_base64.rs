use crate::common::util::*;

#[test]
fn test_encode() {
    let input = "hello, world!";
    new_ucmd!()
        .pipe_in(input)
        .succeeds()
        .stdout_only("aGVsbG8sIHdvcmxkIQ==\n"); // spell-checker:disable-line

    // Using '-' as our file
    new_ucmd!()
        .arg("-")
        .pipe_in(input)
        .succeeds()
        .stdout_only("aGVsbG8sIHdvcmxkIQ==\n"); // spell-checker:disable-line
}

#[test]
fn test_base64_encode_file() {
    new_ucmd!()
        .arg("input-simple.txt")
        .succeeds()
        .stdout_only("SGVsbG8sIFdvcmxkIQo=\n"); // spell-checker:disable-line
}

#[test]
fn test_decode() {
    for decode_param in ["-d", "--decode", "--dec"] {
        let input = "aGVsbG8sIHdvcmxkIQ=="; // spell-checker:disable-line
        new_ucmd!()
            .arg(decode_param)
            .pipe_in(input)
            .succeeds()
            .stdout_only("hello, world!");
    }
}

#[test]
fn test_garbage() {
    let input = "aGVsbG8sIHdvcmxkIQ==\0"; // spell-checker:disable-line
    new_ucmd!()
        .arg("-d")
        .pipe_in(input)
        .fails()
        .stderr_only("base64: error: invalid input\n");
}

#[test]
fn test_ignore_garbage() {
    for ignore_garbage_param in ["-i", "--ignore-garbage", "--ig"] {
        let input = "aGVsbG8sIHdvcmxkIQ==\0"; // spell-checker:disable-line
        new_ucmd!()
            .arg("-d")
            .arg(ignore_garbage_param)
            .pipe_in(input)
            .succeeds()
            .stdout_only("hello, world!");
    }
}

#[test]
fn test_wrap() {
    for wrap_param in ["-w", "--wrap", "--wr"] {
        let input = "The quick brown fox jumps over the lazy dog.";
        new_ucmd!()
            .arg(wrap_param)
            .arg("20")
            .pipe_in(input)
            .succeeds()
            // spell-checker:disable-next-line
            .stdout_only("VGhlIHF1aWNrIGJyb3du\nIGZveCBqdW1wcyBvdmVy\nIHRoZSBsYXp5IGRvZy4=\n");
    }
}

#[test]
fn test_wrap_no_arg() {
    for wrap_param in ["-w", "--wrap"] {
        new_ucmd!().arg(wrap_param).fails().stderr_contains(
            &"The argument '--wrap <wrap>' requires a value but none was supplied",
        );
    }
}

#[test]
fn test_wrap_bad_arg() {
    for wrap_param in ["-w", "--wrap"] {
        new_ucmd!()
            .arg(wrap_param)
            .arg("b")
            .fails()
            .stderr_only("base64: invalid wrap size: 'b'\n");
    }
}

#[test]
fn test_base64_extra_operand() {
    // Expect a failure when multiple files are specified.
    new_ucmd!()
        .arg("a.txt")
        .arg("b.txt")
        .fails()
        .usage_error("extra operand 'b.txt'");
}

#[test]
fn test_base64_file_not_found() {
    new_ucmd!()
        .arg("a.txt")
        .fails()
        .stderr_only("base64: a.txt: No such file or directory");
}
