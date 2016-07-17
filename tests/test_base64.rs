use common::util::*;

static UTIL_NAME: &'static str = "base64";

#[test]
fn test_encode() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    let input = "hello, world!";
    ucmd.pipe_in(input)
        .succeeds()
        .stdout_only("aGVsbG8sIHdvcmxkIQ==\n");
}

#[test]
fn test_decode() {
    for decode_param in vec!["-d", "--decode"] {
        let (_, mut ucmd) = testing(UTIL_NAME);
        let input = "aGVsbG8sIHdvcmxkIQ==";
        ucmd.arg(decode_param)
            .pipe_in(input)
            .succeeds()
            .stdout_only("hello, world!");
    }
}

#[test]
fn test_garbage() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    let input = "aGVsbG8sIHdvcmxkIQ==\0";
    ucmd.arg("-d")
        .pipe_in(input)
        .fails()
        .stderr_only("base64: error: invalid character (Invalid character '0' at position 20)\n");
}

#[test]
fn test_ignore_garbage() {
    for ignore_garbage_param in vec!["-i", "--ignore-garbage"] {
        let (_, mut ucmd) = testing(UTIL_NAME);
        let input = "aGVsbG8sIHdvcmxkIQ==\0";
        ucmd.arg("-d").arg(ignore_garbage_param)
            .pipe_in(input)
            .succeeds()
            .stdout_only("hello, world!");
    }
}

#[test]
fn test_wrap() {
    for wrap_param in vec!["-w", "--wrap"] {
        let (_, mut ucmd) = testing(UTIL_NAME);
        let input = "The quick brown fox jumps over the lazy dog.";
        ucmd.arg(wrap_param).arg("20")
            .pipe_in(input)
            .succeeds()
            .stdout_only("VGhlIHF1aWNrIGJyb3du\nIGZveCBqdW1wcyBvdmVy\nIHRoZSBsYXp5IGRvZy4=\n");
    }
}

#[test]
fn test_wrap_no_arg() {
    for wrap_param in vec!["-w", "--wrap"] {
        let (_, mut ucmd) = testing(UTIL_NAME);
        ucmd.arg(wrap_param)
            .fails()
            .stderr_only(
                format!("base64: error: Argument to option '{}' missing.",
                        if wrap_param == "-w" { "w" } else { "wrap" }));
    }
}

#[test]
fn test_wrap_bad_arg() {
    for wrap_param in vec!["-w", "--wrap"] {
        let (_, mut ucmd) = testing(UTIL_NAME);
        ucmd.arg(wrap_param).arg("b")
            .fails()
            .stderr_only("base64: error: Argument to option 'wrap' improperly formatted: invalid digit found in string");
    }
}
